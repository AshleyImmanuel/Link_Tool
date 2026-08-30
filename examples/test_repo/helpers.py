# helpers.py — utility functions

def add(a, b):
    """Add two numbers."""
    return a + b

def format_result(value):
    """Format a numeric result as string."""
    return f"Result: {value}"

class Calculator:
    def __init__(self):
        self.history = []

    def calculate(self, a, b):
        result = add(a, b)
        self.history.append(result)
        return result

    def show_history(self):
        for entry in self.history:
            print(format_result(entry))
