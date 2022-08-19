use std::rc::Rc;

use crate::proto::{compiler::ts::constants::get_basic_wire_type, package::FieldType};

use super::ast::{self, ForStatement};

pub(super) fn encode_basic_repeated_type_field(
    field_value: &Rc<ast::Expression>,
    field_type: &FieldType,
    field_tag: i64,
    writer_var: &Rc<ast::Identifier>,
) -> ast::Statement {
    let field_exists_expression =
        Rc::new(ast::Expression::BinaryExpression(ast::BinaryExpression {
            operator: ast::BinaryOperator::LogicalAnd,
            left: ast::Expression::BinaryExpression(ast::BinaryExpression {
                operator: ast::BinaryOperator::WeakNotEqual,
                left: Rc::clone(&field_value),
                right: Rc::new(ast::Expression::Null),
            })
            .into(),
            right: ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
                expression: Rc::clone(&field_value),
                name: Rc::new(ast::Identifier::from("length")),
            })
            .into(),
        }));

    let encode_elements_stmt = match field_type {
        FieldType::IdPath(_) => unreachable!(),
        FieldType::Repeated(_) => unreachable!(),
        FieldType::Map(_, _) => unreachable!(),
        basic => match basic.packed_wire_type() {
            Some(_) => encode_packed_elements(&field_value, basic, field_tag, &writer_var),
            None => encode_non_packed_elements(&field_value, basic, field_tag, &writer_var),
        },
    };

    ast::Statement::IfStatement(ast::IfStatement {
        expression: field_exists_expression,
        then_statement: encode_elements_stmt.into(),
        else_statement: None,
    })
}

fn encode_non_packed_elements(
    field_value: &Rc<ast::Expression>,
    element_type: &FieldType,
    field_tag: i64,
    writer_var: &Rc<ast::Identifier>,
) -> ast::Statement {
    assert!(element_type.is_basic());
    let mut res = ast::Block::new();

    let wire_type = get_basic_wire_type(element_type);

    let field_prefix = field_tag << 3 | (wire_type as i64);

    let writer_expr: Rc<ast::Expression> =
        ast::Expression::Identifier(Rc::clone(writer_var)).into();

    let tag_encoding_expr = ast::Expression::CallExpression(ast::CallExpression {
        expression: ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
            expression: Rc::clone(&writer_expr),
            name: Rc::new(ast::Identifier::from("uint32")),
        })
        .into(),
        arguments: vec![Rc::new(ast::Expression::NumericLiteral(
            field_prefix as f64,
        ))],
    });

    let i_id = Rc::new(ast::Identifier::new("i"));
    let i_id_expr = Rc::new(ast::Expression::Identifier(Rc::clone(&i_id)));

    let element_value_expr: Rc<ast::Expression> =
        ast::Expression::ElementAccessExpression(ast::ElementAccessExpression {
            expression: Rc::clone(&field_value),
            argumentExpression: i_id_expr,
        })
        .into();

    let encoding_func_expr: Rc<ast::Expression> =
        ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
            expression: Rc::new(tag_encoding_expr),
            name: Rc::new(ast::Identifier::from(format!("{}", element_type))),
        })
        .into();

    let encode_element_expr: Rc<ast::Expression> =
        ast::Expression::CallExpression(ast::CallExpression {
            expression: encoding_func_expr,
            arguments: vec![element_value_expr],
        })
        .into();

    let mut for_stmt = ForStatement::for_each(i_id, Rc::clone(&field_value));
    for_stmt.push_statement(ast::Statement::Expression(encode_element_expr));

    res.add_statement(Rc::new(ast::Statement::For(for_stmt.into())));

    ast::Statement::Block(res)
}
fn encode_packed_elements(
    field_value: &Rc<ast::Expression>,
    element_type: &FieldType,
    field_tag: i64,
    writer_var: &Rc<ast::Identifier>,
) -> ast::Statement {
    assert!(element_type.is_basic());
    let mut res = ast::Block::new();

    let field_prefix = field_tag << 3 | 2;

    let writer_expr: Rc<ast::Expression> =
        ast::Expression::Identifier(Rc::clone(writer_var)).into();

    let tag_encoding_expr = ast::Expression::CallExpression(ast::CallExpression {
        expression: ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
            expression: Rc::clone(&writer_expr),
            name: Rc::new(ast::Identifier::from("uint32")),
        })
        .into(),
        arguments: vec![Rc::new(ast::Expression::NumericLiteral(
            field_prefix as f64,
        ))],
    });

    let fork_expr: ast::Expression =
        ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
            expression: Rc::new(tag_encoding_expr),
            name: Rc::new(ast::Identifier::from("fork")),
        });

    let fork_call: ast::Expression = ast::Expression::CallExpression(ast::CallExpression {
        expression: Rc::new(fork_expr),
        arguments: vec![],
    })
    .into();

    res.add_statement(ast::Statement::Expression(fork_call.into()).into());

    let i_id = Rc::new(ast::Identifier::new("i"));
    let i_id_expr = Rc::new(ast::Expression::Identifier(Rc::clone(&i_id)));
    let mut for_stmt = ForStatement::for_each(i_id, Rc::clone(&field_value));

    let element_value_expr: Rc<ast::Expression> =
        ast::Expression::ElementAccessExpression(ast::ElementAccessExpression {
            expression: Rc::clone(&field_value),
            argumentExpression: i_id_expr,
        })
        .into();

    let encoding_func_expr: Rc<ast::Expression> =
        ast::Expression::PropertyAccessExpression(ast::PropertyAccessExpression {
            expression: Rc::clone(&writer_expr),
            name: Rc::new(ast::Identifier::from(format!("{}", element_type))),
        })
        .into();

    let encode_element_expr: Rc<ast::Expression> =
        ast::Expression::CallExpression(ast::CallExpression {
            expression: encoding_func_expr,
            arguments: vec![element_value_expr],
        })
        .into();

    for_stmt.push_statement(ast::Statement::Expression(encode_element_expr));

    res.add_statement(Rc::new(ast::Statement::For(for_stmt.into())));

    res.add_statement(Rc::new(ast::Statement::Expression(
        ast::Expression::CallExpression(ast::CallExpression {
            expression: Rc::new(ast::Expression::PropertyAccessExpression(
                ast::PropertyAccessExpression {
                    expression: Rc::clone(&writer_expr),
                    name: Rc::new(ast::Identifier::from("ldelim")),
                },
            )),
            arguments: vec![],
        })
        .into(),
    )));

    ast::Statement::Block(res)
}
