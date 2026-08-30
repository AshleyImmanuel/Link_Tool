// main.js — entry point, uses utils
import { add, multiply, validate } from './utils';

function processOrder(items) {
  validate(items);
  let total = 0;
  for (const item of items) {
    total = add(total, item.price);
  }
  return total;
}

function calculateDiscount(price, percent) {
  const discount = multiply(price, percent / 100);
  return add(price, -discount);
}

const result = processOrder([{ price: 10 }, { price: 20 }]);
console.log(result);
