import type {BoultonFetchableResolver, ReaderAst, FragmentReference} from '@boulton/react';
import { user_detail_page as resolver } from '../user_detail_page.tsx';
import User__user_profile_with_details, { ReadOutType as User__user_profile_with_details__outputType } from './User__user_profile_with_details.boulton';

const queryText = 'query user_detail_page  {\
  current_user {\
    avatar_url,\
    email,\
    id,\
    name,\
    billing_details {\
      address,\
      card_brand,\
      credit_card_number,\
      expiration_date,\
      id,\
    },\
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "current_user",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "user_profile_with_details",
        arguments: null,
        resolver: User__user_profile_with_details,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = {
  current_user: {
    id: string,
    user_profile_with_details: User__user_profile_with_details__outputType,
  },
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = FragmentReference<ReadFromStoreType, ResolverParameterType, ResolverReturnType>;

const artifact: BoultonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: (x) => { return x; },
};

export default artifact;
