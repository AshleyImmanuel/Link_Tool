; Go — symbol definitions, calls, and imports

; Function declarations
(function_declaration
  name: (identifier) @definition.function)

; Method declarations
(method_declaration
  name: (field_identifier) @definition.method)

; Type declarations (struct, interface, alias)
(type_declaration
  (type_spec
    name: (type_identifier) @definition.type))

; Variable declarations
(var_declaration
  (var_spec
    name: (identifier) @definition.variable))

; Short variable declarations
(short_var_declaration
  left: (expression_list
    (identifier) @definition.variable))

; Const declarations
(const_declaration
  (const_spec
    name: (identifier) @definition.variable))

; Function calls
(call_expression
  function: (identifier) @call)

; Method / package function calls
(call_expression
  function: (selector_expression
    field: (field_identifier) @call))

; Import specs
(import_spec
  path: (interpreted_string_literal) @import.source)
