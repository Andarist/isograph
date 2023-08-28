import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
import {getRefRendererForName} from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__pull_request_detail, { ReadOutType as Query__pull_request_detail__outputType } from './pull_request_detail.isograph';

const queryText = 'query pull_request ($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!) {\
  repository____owner___repositoryOwner____name___repositoryName: repository(owner: $repositoryOwner, name: $repositoryName) {\
    id,\
    pullRequest____number___pullRequestNumber: pullRequest(number: $pullRequestNumber) {\
      id,\
      bodyHTML,\
      comments____last___last: comments(last: $last) {\
        edges {\
          node {\
            id,\
            author {\
              login,\
            },\
            bodyText,\
            createdAt,\
          },\
        },\
      },\
      title,\
    },\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: "repository____owner___repositoryOwner____name___repositoryName",
    arguments: [
      {
        argument_name: "owner",
        variable_name: "repositoryOwner",
      },

      {
        argument_name: "name",
        variable_name: "repositoryName",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "pullRequest",
        alias: "pullRequest____number___pullRequestNumber",
        arguments: [
          {
            argument_name: "number",
            variable_name: "pullRequestNumber",
          },
        ],
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "bodyHTML",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "comments",
            alias: "comments____last___last",
            arguments: [
              {
                argument_name: "last",
                variable_name: "last",
              },
            ],
            selections: [
              {
                kind: "Linked",
                fieldName: "edges",
                alias: null,
                arguments: null,
                selections: [
                  {
                    kind: "Linked",
                    fieldName: "node",
                    alias: null,
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "id",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Linked",
                        fieldName: "author",
                        alias: null,
                        arguments: null,
                        selections: [
                          {
                            kind: "Scalar",
                            fieldName: "login",
                            alias: null,
                            arguments: null,
                          },
                        ],
                      },
                      {
                        kind: "Scalar",
                        fieldName: "bodyText",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        fieldName: "createdAt",
                        alias: null,
                        arguments: null,
                      },
                    ],
                  },
                ],
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "avatarUrl",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
    ],
  },
];
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "header",
    arguments: null,
    resolver: Query__header,
    variant: "Component",
  },
  {
    kind: "Resolver",
    alias: "pull_request_detail",
    arguments: null,
    resolver: Query__pull_request_detail,
    variant: "Component",
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  pull_request_detail: Query__pull_request_detail__outputType,
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
};

export default artifact;
