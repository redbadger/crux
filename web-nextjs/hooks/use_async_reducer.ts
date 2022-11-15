import {
  Reducer,
  ReducerAction,
  useState,
} from "react";

export default function useAsyncReducer<R extends Reducer<any, any>, S>(
  reducer: R,
  initState: S
): [S, (action: ReducerAction<R>) => void] {
  const [state, setState] = useState(initState);

  const dispatchState = async (action: ReducerAction<R>) => {
    setState(await reducer(state, action));
  };

  return [state, dispatchState];
}
