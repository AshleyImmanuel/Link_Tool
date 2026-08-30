; JavaScript / JSX — symbol definitions, calls, and imports

; Function declarations
(function_declaration
  name: (identifier) @definition.function)

; Arrow functions assigned to variables
(lexical_declaration
  (variable_declarator
    name: (identifier) @definition.function
    value: (arrow_function)))

(variable_declaration
  (variable_declarator
    name: (identifier) @definition.function
    value: (arrow_function)))

; Class declarations
(class_declaration
  name: (identifier) @definition.class)

; Method definitions inside classes
(method_definition
  name: (property_identifier) @definition.method)

; Variable declarations (non-arrow)
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

; Require calls
(call_expression
  function: (identifier) @_req
  arguments: (arguments (string) @import.source)
  (#eq? @_req "require"))
