import { Arguments } from './index';
import { ReaderArtifact } from './reader';

// This type should be treated as an opaque type.
export type IsographEntrypoint<
  TReadFromStore extends Object,
  TResolverResult,
> = {
  kind: 'Entrypoint';
  queryText: string;
  normalizationAst: NormalizationAst;
  readerArtifact: ReaderArtifact<TReadFromStore, TResolverResult>;
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};

export type NormalizationAstNode =
  | NormalizationScalarField
  | NormalizationLinkedField;
export type NormalizationAst = NormalizationAstNode[];

export type NormalizationScalarField = {
  kind: 'Scalar';
  fieldName: string;
  arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  kind: 'Linked';
  fieldName: string;
  arguments: Arguments | null;
  selections: NormalizationAst;
};

// This is more like an entrypoint, but one specifically for a refetch query/mutation
export type RefetchQueryArtifact = {
  kind: 'RefetchQuery';
  queryText: string;
  normalizationAst: NormalizationAst;
};

// TODO rename
export type RefetchQueryArtifactWrapper = {
  artifact: RefetchQueryArtifact;
  allowedVariables: string[];
};

export function assertIsEntrypoint<
  TReadFromStore extends Object,
  TResolverResult,
>(
  value:
    | IsographEntrypoint<TReadFromStore, TResolverResult>
    | ((_: any) => any)
    // Temporarily, allow any here. Once we automatically provide
    // types to entrypoints, we probably don't need this.
    | any,
): asserts value is IsographEntrypoint<TReadFromStore, TResolverResult> {
  if (typeof value === 'function') throw new Error('Not a string');
}
