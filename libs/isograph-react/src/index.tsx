import { getOrCreateCacheForArtifact, subscribe } from './cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { type PromiseWrapper } from './PromiseWrapper';
import { DataId, DataTypeValue, Link, ROOT_ID } from './IsographEnvironment';
import { useEffect, useState } from 'react';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ReaderArtifact } from './reader';
import {
  IsographEntrypoint,
  RefetchQueryArtifactWrapper,
  assertIsEntrypoint,
} from './entrypoint';
import { read } from './read';

export {
  retainQuery,
  unretainQuery,
  type RetainedQuery,
  garbageCollectEnvironment,
} from './garbageCollection';
export { type PromiseWrapper } from './PromiseWrapper';
export { makeNetworkRequest, subscribe } from './cache';
export {
  ROOT_ID,
  type DataId,
  type DataTypeValue,
  type IsographEnvironment,
  type IsographNetworkFunction,
  type IsographStore,
  type Link,
  type StoreRecord,
  createIsographEnvironment,
  createIsographStore,
  defaultMissingFieldHandler,
} from './IsographEnvironment';
export {
  IsographEnvironmentProvider,
  useIsographEnvironment,
  type IsographEnvironmentProviderProps,
} from './IsographEnvironmentProvider';
export { useImperativeReference } from './useImperativeReference';
export { EntrypointReader } from './EntrypointReader';
export {
  type ReaderArtifact,
  ReaderAst,
  ReaderAstNode,
  ReaderLinkedField,
  ReaderMutationField,
  ReaderRefetchField,
  ReaderResolverField,
  ReaderResolverVariant,
  ReaderScalarField,
} from './reader';
export {
  NormalizationAst,
  NormalizationAstNode,
  NormalizationLinkedField,
  NormalizationScalarField,
  IsographEntrypoint,
  assertIsEntrypoint,
  RefetchQueryArtifact,
  RefetchQueryArtifactWrapper,
} from './entrypoint';
export { read, readButDoNotEvaluate } from './read';

export type ExtractSecondParam<T extends (arg1: any, arg2: any) => any> =
  T extends (arg1: any, arg2: infer P) => any ? P : never;

export type Arguments = Argument[];
export type Argument = [ArgumentName, ArgumentValue];
export type ArgumentName = string;
export type ArgumentValue =
  | {
      kind: 'Variable';
      name: string;
    }
  | {
      kind: 'Literal';
      value: any;
    };

// TODO type this better
type Variable = any;

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverResult,
> = {
  kind: 'FragmentReference';
  readerArtifact: ReaderArtifact<TReadFromStore, TResolverResult>;
  root: DataId;
  variables: { [index: string]: Variable } | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<infer X, any> ? X : never;
export type ExtractResolverResult<Type> =
  Type extends IsographEntrypoint<any, infer X> ? X : never;
// Note: we cannot write TEntrypoint extends IsographEntrypoint<any, any, any>, or else
// if we do not explicitly pass a type, the read out type will be any.
// We cannot write TEntrypoint extends IsographEntrypoint<never, never, never>, or else
// any actual Entrypoint we pass will not be valid.
export function useLazyReference<TEntrypoint>(
  entrypoint:
    | TEntrypoint
    // Temporarily, we need to allow useLazyReference to take the result of calling
    // iso(`...`). At runtime, we confirm that the passed-in `iso` literal is actually
    // an entrypoint.
    | ((_: any) => any),
  variables: { [key: string]: Variable },
): {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >;
} {
  const environment = useIsographEnvironment();
  assertIsEntrypoint<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >(entrypoint);
  const cache = getOrCreateCacheForArtifact<ExtractResolverResult<TEntrypoint>>(
    environment,
    entrypoint,
    variables,
  );

  // TODO add comment explaining why we never use this value
  // @ts-ignore
  const data =
    useLazyDisposableState<PromiseWrapper<ExtractResolverResult<TEntrypoint>>>(
      cache,
    ).state;

  return {
    queryReference: {
      kind: 'FragmentReference',
      readerArtifact: entrypoint.readerArtifact,
      root: ROOT_ID,
      variables,
      nestedRefetchQueries: entrypoint.nestedRefetchQueries,
    },
  };
}

export function useResult<TReadFromStore extends Object, TResolverResult>(
  fragmentReference: FragmentReference<TReadFromStore, TResolverResult>,
): TResolverResult {
  const environment = useIsographEnvironment();

  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(environment, () => {
      return setState({});
    });
  }, []);

  return read(environment, fragmentReference);
}

export function assertLink(link: DataTypeValue): Link | null {
  if (Array.isArray(link)) {
    throw new Error('Unexpected array');
  }
  if (link == null) {
    return null;
  }
  if (typeof link === 'object') {
    return link;
  }
  throw new Error('Invalid link');
}
