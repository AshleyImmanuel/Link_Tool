; Python — symbol definitions, calls, and imports

; Function definitions
(function_definition
  name: (identifier) @definition.function)

; Class definitions
(class_definition
  name: (identifier) @definition.class)

; Variable assignments at module level
(expression_statement
  (assignment
    left: (identifier) @definition.variable))

; Function/method calls
(call
  function: (identifier) @call)

; Method calls
(call
  function: (attribute
    attribute: (identifier) @call))

; from X import Y
(import_from_statement
  name: (dotted_name) @import.source
  (import_prefix)? @_prefix
  name: (dotted_name
    (identifier) @import.name))

; from X import Y (with aliased imports)  
(import_from_statement
  module_name: (dotted_name) @import.source
  name: (aliased_import
    name: (dotted_name
      (identifier) @import.name)))

; import X
(import_statement
  name: (dotted_name
    (identifier) @import.name))

; Decorated definitions (still captured by the inner function/class def above)
