// utils.js — utility functions

export function add(a, b) {
  return a + b;
}

export function multiply(a, b) {
  return a * b;
}

export function validate(value) {
  if (value === null || value === undefined) {
    throw new Error("Invalid value");
  }
  return true;
}

export function log(message) {
  console.log(`[LOG] ${message}`);
}
