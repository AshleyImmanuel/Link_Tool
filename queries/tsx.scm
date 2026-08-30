; TSX â€” TypeScript symbols, calls, imports, and JSX component usage

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

; JSX component usage
(jsx_opening_element
  name: (identifier) @render
  (#match? @render "^[A-Z]"))

(jsx_self_closing_element
  name: (identifier) @render
  (#match? @render "^[A-Z]"))

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

; Express-style routes: app.get("/path", handler) / router.post("/path", handler)
(call_expression
  function: (member_expression
    object: (identifier) @_router
    property: (property_identifier) @route.method
    (#match? @_router "^(app|router)$")
    (#match? @route.method "^(get|post|put|delete|patch|options|head|all)$"))
  arguments: (arguments
    (string) @route.path
    (identifier) @route.handler))
