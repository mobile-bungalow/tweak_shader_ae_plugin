use glsl::parser::Parse;
use glsl::syntax::{
    Declaration, Expr, ExternalDeclaration, FullySpecifiedType, FunctionPrototype,
    InitDeclaratorList, SimpleStatement, SingleDeclaration, TypeQualifier, TypeQualifierSpec,
    TypeSpecifier, TypeSpecifierNonArray,
};
use glsl::visitor::HostMut;
use glsl::visitor::VisitorMut;

const TEXTURE_SAMPLING_FUNCTIONS: [&str; 10] = [
    "texture",
    "textureOffset",
    "textureProj",
    "textureProjOffset",
    "textureLod",
    "textureLodOffset",
    "textureGrad",
    "textureGradOffset",
    "texelFetch",
    "texelGather",
];

const IMAGE_STORE_FUNCTION: [&str; 1] = ["imageStore"];

struct ExitSwizzler {
    pub out_var: String,
}

impl ExitSwizzler {
    pub fn new(out_var: String) -> Self {
        Self { out_var }
    }
}

impl VisitorMut for ExitSwizzler {
    fn visit_compound_statement(
        &mut self,
        block: &mut glsl::syntax::CompoundStatement,
    ) -> glsl::visitor::Visit {
        let mut ret_index = None;
        for (index, statement) in block.statement_list.iter().enumerate() {
            if let glsl::syntax::Statement::Simple(simple) = statement {
                if let SimpleStatement::Jump(glsl::syntax::JumpStatement::Return(_)) = **simple {
                    ret_index = Some(index);
                }
            }
        }
        if let Some(index) = ret_index {
            let out_var = self.out_var.clone();
            let ender = format!("{out_var} = {out_var}.argb;");
            let new_stmnt = glsl::syntax::Statement::parse(ender);
            if let Ok(end_stmnt) = new_stmnt {
                block
                    .statement_list
                    .insert(index.saturating_sub(1), end_stmnt);
            }
        }

        glsl::visitor::Visit::Children
    }
}

struct FormatSwizzler;

impl FormatSwizzler {
    pub fn new() -> Self {
        Self
    }
}

impl VisitorMut for FormatSwizzler {
    fn visit_expr(&mut self, e: &mut Expr) -> glsl::visitor::Visit {
        if let Expr::FunCall(id, args) = e {
            let mut string = String::new();
            glsl::transpiler::glsl::show_function_identifier(&mut string, id);

            if TEXTURE_SAMPLING_FUNCTIONS.contains(&string.as_str()) {
                let clone = e.clone();
                let swizzed = Expr::Dot(clone.into(), glsl::syntax::Identifier("gbar".into()));
                *e = swizzed;
            } else if IMAGE_STORE_FUNCTION.contains(&string.as_str()) {
                if let Some(last_arg) = args.last_mut() {
                    let clone = last_arg.clone();
                    let swizzed = Expr::Dot(clone.into(), glsl::syntax::Identifier("argb".into()));
                    *last_arg = swizzed;
                }
            }

            return glsl::visitor::Visit::Parent;
        }

        glsl::visitor::Visit::Children
    }

    fn visit_translation_unit(
        &mut self,
        tu: &mut glsl::syntax::TranslationUnit,
    ) -> glsl::visitor::Visit {
        let mut exit_swiz = None;

        for item in &mut tu.0 {
            // Check for out vec4 to identify fragment shaders
            if let ExternalDeclaration::Declaration(Declaration::InitDeclaratorList(
                InitDeclaratorList {
                    head:
                        SingleDeclaration {
                            ty:
                                FullySpecifiedType {
                                    qualifier: Some(TypeQualifier { qualifiers }),
                                    ty:
                                        TypeSpecifier {
                                            ty: TypeSpecifierNonArray::Vec4,
                                            array_specifier: None,
                                        },
                                },
                            name: Some(name),
                            array_specifier: None,
                            ..
                        },
                    ..
                },
            )) = item
            {
                if qualifiers.0.contains(&TypeQualifierSpec::Storage(
                    glsl::syntax::StorageQualifier::Out,
                )) {
                    let mut string = String::new();
                    glsl::transpiler::glsl::show_identifier(&mut string, name);
                    exit_swiz = Some(ExitSwizzler::new(string));
                    break;
                }
            }
        }

        if let Some(mut swizzler) = exit_swiz {
            for item in &mut tu.0 {
                if let ExternalDeclaration::FunctionDefinition(glsl::syntax::FunctionDefinition {
                    prototype: FunctionPrototype { name, .. },
                    statement,
                }) = item
                {
                    let mut string = String::new();
                    glsl::transpiler::glsl::show_identifier(&mut string, name);
                    if string == "main" {
                        statement.visit_mut(&mut swizzler);
                        let out_var = swizzler.out_var;
                        let ender = format!("{out_var} = {out_var}.argb;");
                        let new_stmnt = glsl::syntax::Statement::parse(ender);
                        if let Ok(end_stmnt) = new_stmnt {
                            statement.statement_list.push(end_stmnt);
                        }
                        break;
                    }
                }
            }
        }

        glsl::visitor::Visit::Children
    }
}

pub fn convert_output_to_ae_format(module: &str) -> Result<String, ()> {
    let mut swiz = FormatSwizzler::new();
    let mut expr = glsl::syntax::TranslationUnit::parse(module).unwrap();
    expr.visit_mut(&mut swiz);

    let mut output = String::new();
    glsl::transpiler::glsl::show_translation_unit(&mut output, &expr);

    Ok(output)
}
