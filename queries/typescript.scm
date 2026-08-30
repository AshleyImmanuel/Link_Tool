; TypeScript / TSX — symbol definitions, calls, and imports
; Shares most patterns with JavaScript, plus type annotations

; Function declarations
(function_declaration
  name: (identifier) @definition.function)

; Arrow functions assigned to variables
(lexical_declaration
  (variable_declarator
    name: (identifier) @definition.function
    value: (arrow_function)))

; Class declarations
(class_declaration
  name: (type_identifier) @definition.class)

; Method definitions
(method_definition
  name: (property_identifier) @definition.method)

; Interface declarations
(interface_declaration
  name: (type_identifier) @definition.interface)

; Type alias declarations
(type_alias_declaration
  name: (type_identifier) @definition.type)

; Enum declarations
(enum_declaration
  name: (identifier) @definition.enum)

; Variable declarations
(lexical_declaration
  (variable_declarator
    name: (identifier) @definition.variable))

; Function calls
(call_expression
  function: (identifier) @call)

; Method calls
(call_expression
  function: (member_expression
    property: (property_identifier) @call))

; Import specifiers
(import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @import.name)))
  source: (string) @import.source)

; Default imports
(import_statement
  (import_clause
    (identifier) @import.name)
  source: (string) @import.source)
