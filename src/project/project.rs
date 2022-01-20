use std::path::{Path, PathBuf};

use crate::io::get_files;
use crate::{
    compiler::{
        ast::Module,
        diagnostics::Logger,
        lexer::{tokens::Token, LexerError},
        parser::{Parser, ParserContext, ParserError},
        CompilerDisplay, CompilerDisplayError, CompilerError, Source, SourceMap, SourceMapError,
        Span,
    },
    StringId, StringTable,
};

#[derive(Debug)]
pub enum ProjectError {
    NoAstGenerated,
    InvalidPath,
    ParserError(ParserError),
    EmptyProject,
}

impl From<CompilerError<ParserError>> for CompilerError<ProjectError> {
    fn from(parser_ce: CompilerError<ParserError>) -> Self {
        let (span, ie) = parser_ce.take();
        CompilerError::new(span, ProjectError::ParserError(ie))
    }
}

impl CompilerDisplay for ProjectError {
    fn fmt(&self, sm: &SourceMap, st: &StringTable) -> Result<String, CompilerDisplayError> {
        match self {
            ProjectError::NoAstGenerated => Ok("No AST Generated by Parser".into()),
            ProjectError::InvalidPath => Ok("Invalid compilation unit: path was empty".into()),
            ProjectError::ParserError(pe) => pe.fmt(sm, st),
            ProjectError::EmptyProject => Ok("No source code in project".into()),
        }
    }
}

/// Given the path to a source, return the name that should be used
/// for the project.
/// If the path is a file, then return the file name (without extension)
/// If the path is a directory, then return the name of the directory
pub fn get_project_name(src: &Path) -> Result<&str, String> {
    if src.is_file() || src.is_dir() {
        src.file_stem()
            .map(|name| name.to_str())
            .flatten()
            .ok_or("Could not extract name from given path".into())
    } else {
        Err("Given path is neither a directory or a file".into())
    }
}

fn create_module_path<'a>(
    module: &'a mut Module<ParserContext>,
    path: &[StringId],
) -> Option<&'a mut Module<ParserContext>> {
    match path.split_first() {
        Some((head, rest)) => {
            if module.get_module(*head).is_none() {
                let sub = Module::new(*head, ParserContext::new(Span::zero()));
                module.add_module(sub);
            }

            let sub = module
                .get_module_mut(*head)
                .expect("A module with this name was just created and ought to be found");

            if rest.len() == 0 {
                Some(sub)
            } else {
                create_module_path(sub, rest)
            }
        }
        None => None,
    }
}

type Project<T> = Vec<CompilationUnit<T>>;

pub struct CompilationUnit<T> {
    path: Vec<String>,
    data: T,
}

/// Given the location of source file(s) this function will read the file
/// or files and construct the [`SourceMap`] for the project.
///
/// If `src_path` is a directory, this will recursively read every file in
/// that directory and its subdirectories.  If it is a file, it will read
/// only that file.
pub fn build_source_map(
    src_path: &std::path::Path,
    ext: &str,
) -> Result<SourceMap, SourceMapError> {
    let mut sm = SourceMap::new();

    let mut files = get_files(&src_path, ext)?;
    files.sort();
    for file in files {
        sm.add_file(file)?;
    }

    Ok(sm)
}

/// Parses every tokenized compilation unit in the given vector.
/// Each compilation unit is parsed into a module named after the
/// path given in the CompilationUnit and all are added as child
/// modules of a single "root" module.
pub fn parse_project(
    root_module: StringId,
    token_sets: Project<Vec<Token>>,
    source_map: &SourceMap,
    string_table: &StringTable,
    logger: &Logger,
) -> Result<Module<ParserContext>, Vec<CompilerError<ProjectError>>> {
    // The root module spans the entire source code space
    let root_span = source_map.span().ok_or(vec![CompilerError::new(
        Span::zero(),
        ProjectError::EmptyProject,
    )])?;

    let mut root = Module::new(root_module, ParserContext::new(root_span));
    let mut errors = vec![];
    for src_tokens in token_sets {
        match parse_src_tokens(src_tokens, string_table, logger) {
            Ok(ast) => append_module(string_table, &mut root, ast),
            Err(e) => errors.push(e),
        }
    }
    if errors.len() == 0 {
        Ok(root)
    } else {
        Err(errors)
    }
}

/// For each compilation unit in the [`SourceMap`], tokenize, and add to a vector
/// of tokenized compilation units.
pub fn tokenize_source_map(
    sourcemap: &SourceMap,
    src_path: &std::path::Path,
    string_table: &StringTable,
    logger: &Logger,
) -> Result<Vec<CompilationUnit<Vec<Token>>>, Vec<CompilerError<LexerError>>> {
    let mut project_token_sets = vec![];

    // Iterate through each entry in the source map
    for idx in 0..sourcemap.len() {
        let entry = sourcemap.get(idx).unwrap();

        // Derive the logical path within the project
        let module_path = file_path_to_module_path(entry.path(), src_path);

        // Create a compilation Unit from the SourceCharIter
        let src = CompilationUnit {
            path: module_path,
            data: entry.read().unwrap(),
        };

        // Get the Token Set and add to the Vector of token sets
        let tokens = tokenize_source(src, string_table, logger)?;
        project_token_sets.push(tokens);
    }

    Ok(project_token_sets)
}

/// Tokenizes a stream of unicode characters.
fn tokenize_source(
    src: CompilationUnit<Source>,
    string_table: &StringTable,
    logger: &Logger,
) -> Result<CompilationUnit<Vec<Token>>, Vec<CompilerError<LexerError>>> {
    let mut lexer = crate::compiler::Lexer::new(src.data, string_table, logger).unwrap();
    let tokens = lexer.tokenize();
    let (tokens, errors): (
        Vec<std::result::Result<Token, _>>,
        Vec<std::result::Result<Token, _>>,
    ) = tokens.into_iter().partition(|t| t.is_ok());

    if errors.len() == 0 {
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter_map(|t| match t {
                Ok(token) => Some(token),
                Err(_) => None,
            })
            .collect();
        Ok(CompilationUnit {
            path: src.path,
            data: tokens,
        })
    } else {
        let errors: Vec<_> = errors
            .into_iter()
            .filter_map(|t| match t {
                Ok(_) => None,
                Err(msg) => Some(msg),
            })
            .collect();
        Err(errors)
    }
}

/// Takes CompilationUnit which has been tokenized and parses the tokens into
/// an AST.
///
/// The last element of the compilation unit's path (the name of the module derived
/// from the source file name).  Will be removed from the path vector, because it becomes
/// part of the data field (when a module is created with the same name that becomes the
/// parent of all items within the source file).
fn parse_src_tokens(
    src_tokens: CompilationUnit<Vec<Token>>,
    string_table: &StringTable,
    logger: &Logger,
) -> Result<CompilationUnit<Module<ParserContext>>, CompilerError<ProjectError>> {
    let parser = Parser::new(logger);
    if let Some((name, parent_path)) = src_tokens.path.split_last() {
        let name = string_table.insert(name.into());
        match parser.parse(name, &src_tokens.data) {
            Ok(Some(ast)) => Ok(CompilationUnit {
                path: parent_path.to_owned(),
                data: ast,
            }),
            Ok(None) => Err(CompilerError::new(
                Span::zero(),
                ProjectError::NoAstGenerated,
            )),
            Err(msg) => Err(msg.into()),
        }
    } else {
        Err(CompilerError::new(
            Span::zero(), // TODO: What's the correct span to have here
            ProjectError::InvalidPath,
        ))
    }
}

fn append_module(
    string_table: &StringTable,
    root: &mut Module<ParserContext>,
    src_ast: CompilationUnit<Module<ParserContext>>,
) {
    let parent = if src_ast.path.len() == 0 {
        root
    } else {
        let path: Vec<_> = src_ast
            .path
            .iter()
            .map(|p| string_table.insert(p.into()))
            .collect();
        create_module_path(root, &path).unwrap()
    };
    parent.add_module(src_ast.data);
}

fn file_path_to_module_path(file: &PathBuf, src_path: &Path) -> Vec<String> {
    let fpath = file.as_path();
    let base = if src_path.is_dir() {
        src_path
    } else {
        src_path
            .parent()
            .expect("Given a file which is also the root of the directory structure.")
    };

    let rel_path = fpath.strip_prefix(&base).unwrap();

    let mut p: Vec<String> = rel_path
        .iter()
        .map(|e| e.to_str().expect("File name was not valid unicode").into())
        .collect();

    truncate_extension(&mut p, ".br");
    p
}

fn truncate_extension(path: &mut Vec<String>, ext: &str) {
    match path.last_mut() {
        Some(l) if l.ends_with(ext) => l.truncate(l.len() - ext.len()),
        _ => (),
    }
}
