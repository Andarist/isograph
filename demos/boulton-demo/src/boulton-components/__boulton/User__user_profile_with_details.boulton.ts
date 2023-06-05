import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_profile_with_details as resolver } from '../user_detail_page.tsx';
import User__avatar_component from './User__avatar_component.boulton';
import BillingDetails__billing_details_component from './BillingDetails__billing_details_component.boulton';

const readerAst: ReaderAst = [
  {
    kind: "Scalar",
    response_name: "id",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "name",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "email",
    alias: null,
  },
  {
    kind: "Resolver",
    alias: "avatar_component",
    resolver: User__avatar_component,
    variant: "Component",
  },
  {
    kind: "Linked",
    response_name: "billing_details",
    alias: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
      },
      {
        kind: "Resolver",
        alias: "billing_details_component",
        resolver: BillingDetails__billing_details_component,
        variant: "Component",
      },
    ],
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
};

export default artifact;
