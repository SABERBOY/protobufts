use crate::proto::package::FieldDeclaration;

use super::{
    error::{syntax_error, ProtoError},
    lexems::{Lexem, LocatedLexem},
    package::{
        Declaration, EnumDeclaration, EnumEntry, FieldType, MessageDeclaration, MessageEntry,
        OneOfDeclaration, Package,
    },
};

#[derive(Debug, Clone)]
enum Task {
    ParseStatements,
    ParseStatement,
    ParseSyntaxStatement,
    ParseImportStatement,
    ParsePackageStatement,
    /// Parses enum declaration and pushes to stack
    ParseEnumDeclaration,
    ParseEnumEntries,
    ParseEnumEntry,
    WrapRepeated,
    ParseFieldDeclaration,
    ParseMessageStatement,
    ExpectLexem(Lexem),
    Push(StackItem),
    ParseFieldAttributes,
    /// Takes declaration from the stack
    /// And pushes it to the package declarations
    AppendDeclarationToPackage,
    ParseFieldAttribute,
    PushFieldDeclaration,
    PushFieldAttribute,
    ParseMessageEntries,
    ParseMessageEntry,
    ParseOptionalAttributes,
    ParseInt64,
    ParseFieldType,
    ParseStringLiteral,
    WrapMessageEntry,
    PushMessageEntry,
    PushMessageStatement,
    ParseIdPath,
    WrapFieldType,
    /// [FieldType, FieldType] => Map<FieldType, FieldType>
    WrapMapType,
    /// Input: Vec<MessageEntries> String
    /// Output: OneOfDeclaration
    PushOneOf,
    /// Parses identifier and places it into stack
    ParseId,
}
use Task::*;

#[derive(Debug, Clone)]
enum StackItem {
    String(String),
    StringList(Vec<String>),
    EnumEntriesList(Vec<EnumEntry>),
    MessageEntriesList(Vec<MessageEntry>),
    MessageEntry(MessageEntry),
    FieldType(FieldType),
    Int64(i64),
    Message(MessageDeclaration),
    OptionalAttributes(Option<Vec<(String, String)>>),
    Enum(EnumDeclaration),
    OneOf(OneOfDeclaration),
}

pub(super) fn parse_package(located_lexems: &[LocatedLexem]) -> Result<Package, ProtoError> {
    let mut ind = 0;
    let mut tasks: Vec<Task> = vec![ParseStatements];
    let mut res = Package {
        version: super::package::ProtoVersion::Proto2,
        declarations: vec![],
        imports: vec![],
        path: vec![],
    };
    let mut stack: Vec<StackItem> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            ParseStatements => {
                let located_lexem = &located_lexems[ind];
                let lexem = &located_lexem.lexem;
                match lexem {
                    Lexem::EOF => {
                        break;
                    }
                    _ => {
                        tasks.push(ParseStatements);
                        tasks.push(ParseStatement)
                    }
                }
            }
            ParseStatement => {
                let located_lexem = &located_lexems[ind];
                let lexem = &located_lexem.lexem;
                match lexem {
                    Lexem::Id(id) if id == "syntax" => {
                        tasks.push(ParseSyntaxStatement);
                        continue;
                    }
                    Lexem::Id(id) if id == "message" => {
                        tasks.push(AppendDeclarationToPackage);
                        tasks.push(ParseMessageStatement);
                        continue;
                    }
                    Lexem::Id(id) if id == "import" => {
                        tasks.push(ParseImportStatement);
                        continue;
                    }
                    Lexem::Id(id) if id == "package" => {
                        tasks.push(ParsePackageStatement);
                        continue;
                    }
                    Lexem::Id(id) if id == "enum" => {
                        tasks.push(AppendDeclarationToPackage);
                        tasks.push(ParseEnumDeclaration);
                        continue;
                    }
                    Lexem::Id(id) => {
                        return Err(syntax_error(
                            format!("Unexpected identifier: {}", id),
                            located_lexem,
                        ));
                    }
                    Lexem::SemiColon => {
                        ind+=1;
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        return Err(syntax_error(
                            format!("Unexpected lexem {:?}", lexem),
                            located_lexem,
                        ));
                    }
                }
            }
            AppendDeclarationToPackage => {
                let declaration = match stack.pop() {
                    Some(StackItem::Message(message)) => Declaration::Message(message),
                    Some(StackItem::Enum(enum_decl)) => Declaration::Enum(enum_decl),
                    _ => unreachable!(),
                };
                res.declarations.push(declaration);
                continue;
            }
            WrapMapType => {
                let value_type = match stack.pop() {
                    Some(StackItem::FieldType(field_type)) => field_type,
                    _ => unreachable!(),
                };
                let key_type = match stack.pop() {
                    Some(StackItem::FieldType(field_type)) => field_type,
                    _ => unreachable!(),
                };
                let map_type = FieldType::Map(Box::new(key_type), Box::new(value_type));
                stack.push(StackItem::FieldType(map_type));
                continue;
            }
            ParseSyntaxStatement => {
                assert_enough_length(
                    located_lexems,
                    ind,
                    4,
                    "Not enough lexems for syntax statement",
                )?;
                let syntax = &located_lexems[ind].lexem;
                let equals = &located_lexems[ind + 1].lexem;
                let version = &located_lexems[ind + 2].lexem;
                let semi_colon = &located_lexems[ind + 3].lexem;
                match (syntax, equals, version, semi_colon) {
                    (Lexem::Id(id), Lexem::Equal, Lexem::StringLiteral(s), Lexem::SemiColon)
                        if id == "syntax" && s == "proto2" =>
                    {
                        ind += 4;
                        continue;
                    }
                    (Lexem::Id(id), Lexem::Equal, Lexem::StringLiteral(s), Lexem::SemiColon)
                        if id == "syntax" && s == "proto3" =>
                    {
                        ind += 4;
                        res.version = super::package::ProtoVersion::Proto3;
                        continue;
                    }
                    _ => {
                        return Err(syntax_error(
                            "Invalid syntax statement",
                            &located_lexems[ind],
                        ));
                    }
                }
            }
            ParseImportStatement => {
                assert_enough_length(
                    located_lexems,
                    ind,
                    3,
                    "Not enough lexems for import statement",
                )?;
                let import = &located_lexems[ind].lexem;
                let str = &located_lexems[ind + 1].lexem;
                let semi_colon = &located_lexems[ind + 2].lexem;
                match (import, str, semi_colon) {
                    (Lexem::Id(id), Lexem::StringLiteral(s), Lexem::SemiColon)
                        if id == "import" =>
                    {
                        ind += 3;
                        res.imports.push(s.clone());
                        continue;
                    }
                    _ => {
                        return Err(syntax_error(
                            "Invalid import statement",
                            &located_lexems[ind],
                        ));
                    }
                }
            }
            ParseInt64 => {
                let located_lexem = &located_lexems[ind];
                let lexem = &located_lexem.lexem;
                match lexem {
                    Lexem::IntLiteral(i) => {
                        stack.push(StackItem::Int64(*i));
                        ind += 1;
                        continue;
                    }
                    _ => {
                        return Err(syntax_error("Expected int literal", located_lexem));
                    }
                }
            }
            ParseOptionalAttributes => {
                let located_lexem = &located_lexems[ind];
                let lexem = &located_lexem.lexem;
                match lexem {
                    Lexem::OpenBracket => {
                        stack.push(StackItem::OptionalAttributes(Some(Vec::new())));
                        tasks.push(ParseFieldAttributes);
                        tasks.push(ExpectLexem(Lexem::OpenBracket));
                        continue;
                    }
                    _ => {
                        stack.push(StackItem::OptionalAttributes(None));
                        continue;
                    }
                }
            }
            ParseFieldAttributes => {
                let located_lexem = &located_lexems[ind];
                let lexem = &located_lexem.lexem;
                if let Lexem::CloseBracket = lexem {
                    ind += 1;
                    continue;
                }
                tasks.push(ParseFieldAttributes);
                tasks.push(ParseFieldAttribute);

                continue;
            }
            ParseFieldAttribute => {
                tasks.push(PushFieldAttribute);
                tasks.push(ParseStringLiteral);
                tasks.push(ExpectLexem(Lexem::Equal));
                tasks.push(ParseId);
                continue;
            }
            PushFieldAttribute => {
                let value = match stack.pop() {
                    Some(StackItem::String(s)) => s,
                    _ => unreachable!(),
                };
                let key = match stack.pop() {
                    Some(StackItem::String(s)) => s,
                    invalid_item => {
                        println!("invalid item = {:?}", invalid_item);
                        println!("value = {:?}", value);
                        println!("location = {:?}", located_lexems[ind].range.start);
                        print_state(stack, tasks, task, &located_lexems[ind - 1..]);
                        unreachable!();
                    }
                };
                let optional_list_item = stack.pop().unwrap();
                let mut optional_list = match optional_list_item {
                    StackItem::OptionalAttributes(Some(optional_list)) => optional_list,
                    _ => unreachable!(),
                };
                optional_list.push((key, value));
                stack.push(StackItem::OptionalAttributes(Some(optional_list)));
                continue;
            }
            PushFieldDeclaration => {
                let attributes = match stack.pop() {
                    Some(StackItem::OptionalAttributes(optional_attributes)) => optional_attributes,
                    _ => unreachable!(),
                }
                .unwrap_or_default();
                let tag = match stack.pop() {
                    Some(StackItem::Int64(tag)) => tag,
                    _ => unreachable!(),
                };
                let name = match stack.pop() {
                    Some(StackItem::String(name)) => name,
                    _ => unreachable!(),
                };
                let field_type = match stack.pop() {
                    Some(StackItem::FieldType(field_type)) => field_type,
                    _ => unreachable!(),
                };
                let field_declaration = FieldDeclaration {
                    name,
                    tag,
                    field_type,
                    attributes,
                };
                let mut message_entries = match stack.pop() {
                    Some(StackItem::MessageEntriesList(list)) => list,
                    _ => unreachable!(),
                };
                message_entries.push(MessageEntry::Field(field_declaration));
                stack.push(StackItem::MessageEntriesList(message_entries));
                continue;
            }
            ParsePackageStatement => {
                assert_enough_length(
                    located_lexems,
                    ind,
                    3,
                    "Not enough lexems for package statement",
                )?;
                let package = &located_lexems[ind].lexem;
                match package {
                    Lexem::Id(id) if id == "package" => {}
                    _ => {
                        return Err(syntax_error(
                            "Invalid package statement",
                            &located_lexems[ind],
                        ));
                    }
                }
                ind += 1;
                res.path = Vec::new();
                'listLoop: loop {
                    let id_loc_lexem = &located_lexems[ind];
                    ind += 1;
                    let id = &id_loc_lexem.lexem;
                    match id {
                        Lexem::Id(id) => {
                            res.path.push(id.clone());
                        }
                        _ => {
                            return Err(syntax_error("Expected identifier", id_loc_lexem));
                        }
                    }
                    let punct_loc_lexem = &located_lexems[ind];
                    ind += 1;
                    let punct = &punct_loc_lexem.lexem;
                    match punct {
                        Lexem::Dot => {
                            continue 'listLoop;
                        }
                        Lexem::SemiColon => {
                            break 'listLoop;
                        }
                        _ => {
                            return Err(syntax_error("Expected dot or semicolon", punct_loc_lexem));
                        }
                    }
                }
                continue;
            }
            ParseEnumDeclaration => {
                assert_enough_length(
                    located_lexems,
                    ind,
                    4,
                    "Not enough lexems for enum statement",
                )?;

                ind += 1;
                let name_loc_lexem = &located_lexems[ind];
                let name = &name_loc_lexem.lexem;
                match name {
                    Lexem::Id(id) => stack.push(StackItem::String(id.clone())),
                    _ => return Err(syntax_error("Expacted enum name", name_loc_lexem)),
                }
                ind += 1;
                let curly_open_loc = &located_lexems[ind];
                let curly_open = &curly_open_loc.lexem;
                match curly_open {
                    Lexem::OpenCurly => {}
                    _ => {
                        return Err(syntax_error("Expected curly open", curly_open_loc));
                    }
                }
                ind += 1;
                stack.push(StackItem::EnumEntriesList(Vec::new()));
                tasks.push(ParseEnumEntries);
                continue;
            }
            ParseEnumEntries => {
                let loc_separator = &located_lexems[ind];
                let separator = &loc_separator.lexem;
                match separator {
                    Lexem::CloseCurly => {
                        ind += 1;
                        let list_item = stack.pop().unwrap();
                        let enum_name_item = stack.pop().unwrap();
                        match (list_item, enum_name_item) {
                            (StackItem::EnumEntriesList(entries), StackItem::String(name)) => {
                                let enum_declaration = EnumDeclaration {
                                    name: name,
                                    entries: entries,
                                };
                                stack.push(StackItem::Enum(enum_declaration));
                            }
                            (a, b) => {
                                println!("Invalid stack items for enum declaration finishing: {:?} and {:?}", a, b);
                                print_state(stack, tasks, task, &located_lexems[ind..]);
                                todo!("Cannot handle separator {:?}", separator);
                            }
                        }
                    }
                    Lexem::Id(_) => {
                        tasks.push(ParseEnumEntries);
                        tasks.push(ParseEnumEntry);
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot handle separator {:?}", separator);
                    }
                }
            }
            ParseEnumEntry => {
                assert_enough_length(located_lexems, ind, 4, "Not enough lexems for enum entry")?;
                let id_loc = &located_lexems[ind];
                ind += 1;
                let eq_loc = &located_lexems[ind];
                ind += 1;
                let value_loc = &located_lexems[ind];
                ind += 1;
                let semi_loc = &located_lexems[ind];
                ind += 1;
                match (
                    &id_loc.lexem,
                    &eq_loc.lexem,
                    &value_loc.lexem,
                    &semi_loc.lexem,
                ) {
                    (Lexem::Id(id), Lexem::Equal, Lexem::IntLiteral(value), Lexem::SemiColon) => {
                        let entries = stack.pop().unwrap();
                        match entries {
                            StackItem::EnumEntriesList(mut list) => {
                                list.push(super::package::EnumEntry {
                                    name: id.clone(),
                                    value: *value,
                                });
                                stack.push(StackItem::EnumEntriesList(list));
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot parse enum entry")
                    }
                }
            }
            ParseMessageStatement => {
                tasks.push(PushMessageStatement);
                tasks.push(ExpectLexem(Lexem::CloseCurly));
                tasks.push(ParseMessageEntries);
                tasks.push(Push(StackItem::MessageEntriesList(Vec::new())));
                tasks.push(ExpectLexem(Lexem::OpenCurly));
                tasks.push(ParseId);
                tasks.push(ExpectLexem(Lexem::Id("message".into())));
                continue;
            }
            Push(stack_item) => {
                stack.push(stack_item);
                continue;
            }
            PushMessageStatement => {
                let entries = match stack.pop() {
                    Some(StackItem::MessageEntriesList(entries)) => entries,
                    _ => unreachable!(),
                };
                let message_name = match stack.pop() {
                    Some(StackItem::String(name)) => name,
                    _ => unreachable!(),
                };
                let message_declaration = MessageDeclaration {
                    name: message_name,
                    entries,
                };
                stack.push(StackItem::Message(message_declaration));
                continue;
            }
            WrapMessageEntry => {
                let entry = match stack.pop() {
                    Some(StackItem::Message(message_declaration)) => {
                        MessageEntry::Message(message_declaration)
                    }
                    Some(StackItem::Enum(enum_declaration)) => MessageEntry::Enum(enum_declaration),
                    Some(StackItem::OneOf(decl)) => MessageEntry::OneOf(decl),
                    _ => unreachable!(),
                };
                stack.push(StackItem::MessageEntry(entry));
                continue;
            }
            PushMessageEntry => {
                let message_entry = match stack.pop() {
                    Some(StackItem::MessageEntry(entry)) => entry,
                    _ => unreachable!(),
                };
                let mut entries = match stack.pop() {
                    Some(StackItem::MessageEntriesList(entries)) => entries,
                    _ => unreachable!(),
                };
                entries.push(message_entry);
                stack.push(StackItem::MessageEntriesList(entries));
                continue;
            }
            ParseMessageEntries => {
                let loc_separator = &located_lexems[ind];
                let separator = &loc_separator.lexem;
                match separator {
                    Lexem::Id(_) => {
                        tasks.push(ParseMessageEntries);
                        tasks.push(ParseMessageEntry);
                        continue;
                    }
                    Lexem::CloseCurly => {
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot handle separator {:?}", separator);
                    }
                }
            }
            ParseMessageEntry => {
                let start_loc = &located_lexems[ind];
                let start = &start_loc.lexem;
                match start {
                    Lexem::Id(id) if id == "message" => {
                        tasks.push(PushMessageEntry);
                        tasks.push(WrapMessageEntry);
                        tasks.push(ParseMessageStatement);
                        continue;
                    }
                    Lexem::Id(id) if id == "enum" => {
                        tasks.push(PushMessageEntry);
                        tasks.push(WrapMessageEntry);
                        tasks.push(ParseEnumDeclaration);
                        continue;
                    }
                    Lexem::Id(id) if id == "oneof" => {
                        tasks.push(PushMessageEntry);
                        tasks.push(WrapMessageEntry);
                        tasks.push(PushOneOf);
                        tasks.push(ExpectLexem(Lexem::CloseCurly));
                        tasks.push(ParseMessageEntries);
                        tasks.push(ExpectLexem(Lexem::OpenCurly));
                        tasks.push(Push(StackItem::MessageEntriesList(Vec::new())));
                        tasks.push(ParseId);
                        tasks.push(ExpectLexem(Lexem::Id("oneof".into())));
                        continue;
                    }
                    Lexem::Id(id) if id == "enum" => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot handle start message entry {:?}", start)
                    }
                    Lexem::Id(_) => {
                        tasks.push(ParseFieldDeclaration);
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot handle start message entry {:?}", start)
                    }
                }
            }
            PushOneOf => {
                let message_entries = match stack.pop() {
                    Some(StackItem::MessageEntriesList(entries)) => entries,
                    _ => unreachable!(),
                };
                let one_of_name = match stack.pop() {
                    Some(StackItem::String(name)) => name,
                    _ => unreachable!(),
                };
                if message_entries.iter().any(|entry| match entry {
                    MessageEntry::Field(_) => false,
                    _ => true,
                }) {
                    return Err(syntax_error(
                        "oneof can contain only field declarations",
                        &located_lexems[ind],
                    ));
                }
                let one_of_declaration = OneOfDeclaration {
                    name: one_of_name,
                    options: message_entries
                        .iter()
                        .filter_map(|entry| match entry {
                            MessageEntry::Field(field_decl) => Some(field_decl.to_owned()),
                            _ => None,
                        })
                        .collect::<Vec<FieldDeclaration>>(),
                };
                stack.push(StackItem::OneOf(one_of_declaration));
                continue;
            }
            ParseFieldType => {
                let start_loc = &located_lexems[ind];
                let start = &start_loc.lexem;
                if let Lexem::Id(id) = start {
                    if id == "repeated" {
                        tasks.push(WrapRepeated);
                        tasks.push(ParseFieldType);
                        ind += 1;
                        continue;
                    }
                    if id == "map" {
                        tasks.push(WrapMapType);
                        tasks.push(ExpectLexem(Lexem::Greater));
                        tasks.push(ParseFieldType);
                        tasks.push(ExpectLexem(Lexem::Comma));
                        tasks.push(ParseFieldType);
                        tasks.push(ExpectLexem(Lexem::Less));
                        tasks.push(ExpectLexem(Lexem::Id("map".into())));
                        continue;
                    }
                    tasks.push(WrapFieldType);
                    tasks.push(ParseIdPath);
                    continue;
                }
                return Err(syntax_error("Expected lexem", start_loc));
            }
            WrapFieldType => {
                let field_type = match stack.pop() {
                    Some(StackItem::StringList(ids)) => FieldType::IdPath(ids),
                    _ => unreachable!(),
                };
                stack.push(StackItem::FieldType(field_type));
                continue;
            }
            ParseIdPath => {
                let mut id_path = Vec::new();
                loop {
                    let id_loc_lexem = &located_lexems[ind];
                    ind += 1;
                    let id = &id_loc_lexem.lexem;
                    match id {
                        Lexem::Id(id) => {
                            id_path.push(id.clone());
                        }
                        _ => {
                            return Err(syntax_error("Expected identifier", id_loc_lexem));
                        }
                    }
                    let punct_loc_lexem = &located_lexems[ind];
                    let punct = &punct_loc_lexem.lexem;
                    match punct {
                        Lexem::Dot => {
                            ind += 1;
                            continue;
                        }
                        _ => {
                            break;
                        }
                    }
                }
                stack.push(StackItem::StringList(id_path));
                continue;
            }
            WrapRepeated => {
                let item = stack.pop();
                match item {
                    Some(StackItem::FieldType(field_type)) => {
                        stack.push(StackItem::FieldType(FieldType::Repeated(Box::new(
                            field_type,
                        ))));
                        continue;
                    }
                    _ => {
                        print_state(stack, tasks, task, &located_lexems[ind..]);
                        todo!("Cannot handle repeated field type")
                    }
                }
            }
            ParseFieldDeclaration => {
                tasks.push(PushFieldDeclaration);
                tasks.push(ExpectLexem(Lexem::SemiColon));
                tasks.push(ParseOptionalAttributes);
                tasks.push(ParseInt64);
                tasks.push(ExpectLexem(Lexem::Equal));
                tasks.push(ParseId);
                tasks.push(ParseFieldType);
                continue;
            }
            ExpectLexem(expected_lexem) => {
                assert_enough_length(
                    located_lexems,
                    ind,
                    1,
                    format!("Expected lexem: {:?}", expected_lexem),
                )?;
                let loc_lexem = &located_lexems[ind];
                let lexem = &loc_lexem.lexem;
                if lexem == &expected_lexem {
                    ind += 1;
                    continue;
                }
                return Err(syntax_error(
                    format!("Expected lexem: {:?}", expected_lexem),
                    loc_lexem,
                ));
            }
            ParseId => {
                assert_enough_length(located_lexems, ind, 1, "Expected identifier")?;
                let loc_lexem = &located_lexems[ind];
                let lexem = &loc_lexem.lexem;
                match lexem {
                    Lexem::Id(found_id) => {
                        ind += 1;
                        stack.push(StackItem::String(found_id.clone()));
                        continue;
                    }
                    _ => {
                        return Err(syntax_error("expected identifier", loc_lexem));
                    }
                }
            }
            ParseStringLiteral => {
                assert_enough_length(located_lexems, ind, 1, "Expected identifier")?;
                let loc_lexem = &located_lexems[ind];
                let lexem = &loc_lexem.lexem;
                match lexem {
                    Lexem::StringLiteral(found_literal) => {
                        ind += 1;
                        stack.push(StackItem::String(found_literal.clone()));
                        continue;
                    }
                    _ => {
                        return Err(syntax_error("expected identifier", loc_lexem));
                    }
                }
            }
            _ => {
                print_state(stack, tasks, task, &located_lexems[ind..]);
                todo!("Cannot solve task")
            }
        }
    }
    Ok(res)
}

fn print_stack(stack: &[StackItem]) {
    println!("\nStack: ");
    println!(
        "{}",
        stack
            .iter()
            .rev()
            .map(|x| match x {
                StackItem::String(_) => "string",
                StackItem::StringList(_) => "string[]",
                StackItem::FieldType(_) => "type",
                StackItem::EnumEntriesList(_) => "EnumEntry[]",
                StackItem::MessageEntriesList(_) => "MessageEntry[]",
                StackItem::MessageEntry(_) => "MessageEntry",
                StackItem::Int64(_) => "i64",
                StackItem::Message(_) => "message",
                StackItem::OptionalAttributes(_) => "attributes[]?",
                StackItem::Enum(_) => "enum",
                StackItem::OneOf(_) => "oneof",
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn print_state(
    mut stack: Vec<StackItem>,
    tasks: Vec<Task>,
    task: Task,
    located_lexems: &[LocatedLexem],
) {
    if stack.len() > 0 {
        println!("Stack:");
        for item in stack.iter().rev() {
            println!("{:#?}", item);
        }
        println!();
        print_stack(&stack);
        println!();
    } else {
        println!("Stack: empty");
        println!();
    }
    if tasks.len() > 0 {
        println!("Tasks:");
        for task in tasks {
            println!("{:?}", task);
        }
        println!("{:?} - current", task);
        println!();
    } else {
        println!("Tasks: empty");
    }

    if located_lexems.is_empty() {
        return;
    }
    println!("Next lexems:");
    for i in 0..located_lexems.len().min(10) {
        if i > 0 {
            let prev = &located_lexems[i - 1].lexem;
            match prev {
                Lexem::CloseCurly | Lexem::SemiColon | Lexem::OpenCurly => {
                    print!("\n")
                }

                _ => match &located_lexems[i].lexem {
                    Lexem::SemiColon => {}
                    _ => {
                        print!(" ")
                    }
                },
            }
        }
        print!("{}", located_lexems[i].lexem);
    }
    println!("\n");
    println!("source: {:?}\n", located_lexems[0].range.start);
}

fn assert_enough_length<M>(
    located_lexems: &[LocatedLexem],
    ind: usize,
    len: usize,
    message: M,
) -> Result<(), ProtoError>
where
    M: Into<String>,
{
    if ind + len - 1 >= located_lexems.len() {
        return Err(syntax_error(message, &located_lexems[ind]));
    }
    return Ok(());
}