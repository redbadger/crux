import {
  RandomNumber,
  RandomNumberRequest,
} from "shared_types/types/shared_types";

export function request(data: RandomNumberRequest): RandomNumber {
  const min = Number(data.field0);
  const max = Number(data.field1);

  let result = Math.floor(Math.random() * (max - min)) + min;

  return new RandomNumber(BigInt(result));
}
