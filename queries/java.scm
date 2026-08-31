; Java queries for Link

; Classes
(class_declaration
  name: (identifier) @symbol.name) @symbol.def

; Interfaces
(interface_declaration
  name: (identifier) @symbol.name) @symbol.def

; Methods
(method_declaration
  name: (identifier) @symbol.name) @symbol.def

; Method Calls
(method_invocation
  name: (identifier) @call.name) @call

; Imports
(import_declaration
  (scoped_identifier) @import.name) @import
