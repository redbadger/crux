import { RandomNumber, RandomNumberRequest } from "app/app";

export function request(data: RandomNumberRequest): RandomNumber {
  const min = Number(data.field0);
  const max = Number(data.field1);

  let result = Math.floor(Math.random() * (max - min)) + min;

  return new RandomNumber(BigInt(result));
}
