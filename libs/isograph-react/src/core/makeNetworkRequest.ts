import { ItemCleanupPair } from '@isograph/disposable-types';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifact,
  type NormalizationAst,
  type NormalizationAstLoader,
} from './entrypoint';
import { ExtractParameters } from './FragmentReference';
import {
  garbageCollectEnvironment,
  RetainedQuery,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import { IsographEnvironment, Link, ROOT_ID } from './IsographEnvironment';
import {
  AnyError,
  PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { normalizeData } from './cache';
import { logMessage } from './logging';
import {
  check,
  DEFAULT_SHOULD_FETCH_VALUE,
  FetchOptions,
  type RequiredFetchOptions,
} from './check';
import { readButDoNotEvaluate } from './read';
import { getOrCreateCachedComponent } from './componentCache';
let networkRequestId = 0;

export function maybeMakeNetworkRequest<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue, NormalizationAst>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>>;
export function maybeMakeNetworkRequest<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        NormalizationAstLoader
      >,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions: RequiredFetchOptions<TClientFieldValue>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>>;
export function maybeMakeNetworkRequest<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  switch (fetchOptions?.shouldFetch ?? DEFAULT_SHOULD_FETCH_VALUE) {
    case 'Yes': {
      return makeNetworkRequest(environment, artifact, variables, fetchOptions);
    }
    case 'No': {
      return [wrapResolvedValue(undefined), () => {}];
    }
    case 'IfNecessary': {
      if (
        artifact.networkRequestInfo.normalizationAst.kind ===
        'NormalizationAstLoader'
      ) {
        throw new Error(
          'Using lazy loaded normalizationAst with shouldFetch: "IfNecessary" is not supported as it will lead to slower initial load time.',
        );
      }
      const result = check(
        environment,
        artifact.networkRequestInfo.normalizationAst.selections,
        variables,
        {
          __link: ROOT_ID,
          __typename: artifact.concreteType,
        },
      );

      if (result.kind === 'EnoughData') {
        return [wrapResolvedValue(undefined), () => {}];
      } else {
        return makeNetworkRequest(
          environment,
          artifact,
          variables,
          fetchOptions,
        );
      }
    }
  }
}

function loadNormalizationAst(
  normalizationAst: NormalizationAstLoader | NormalizationAst,
) {
  switch (normalizationAst.kind) {
    case 'NormalizationAst': {
      return normalizationAst;
    }
    case 'NormalizationAstLoader': {
      return normalizationAst.loader();
    }
  }
}

export function makeNetworkRequest<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  // TODO this should be a DataId and stored in the store
  const myNetworkRequestId = networkRequestId + '';
  networkRequestId++;

  logMessage(environment, {
    kind: 'MakeNetworkRequest',
    artifact,
    variables,
    networkRequestId: myNetworkRequestId,
  });

  let status: NetworkRequestStatus = {
    kind: 'UndisposedIncomplete',
  };
  // This should be an observable, not a promise
  const promise = Promise.all([
    environment.networkFunction(
      artifact.networkRequestInfo.queryText,
      variables,
    ),
    loadNormalizationAst(artifact.networkRequestInfo.normalizationAst),
  ])
    .then(([networkResponse, normalizationAst]) => {
      logMessage(environment, {
        kind: 'ReceivedNetworkResponse',
        networkResponse,
        networkRequestId: myNetworkRequestId,
      });

      if (networkResponse.errors != null) {
        try {
          fetchOptions?.onError?.();
        } catch {}
        // @ts-expect-error Why are we getting the wrong constructor here?
        throw new Error('GraphQL network response had errors', {
          cause: networkResponse,
        });
      }

      const root = { __link: ROOT_ID, __typename: artifact.concreteType };
      if (status.kind === 'UndisposedIncomplete') {
        normalizeData(
          environment,
          normalizationAst.selections,
          networkResponse.data ?? {},
          variables,
          artifact.kind === 'Entrypoint'
            ? artifact.readerWithRefetchQueries.nestedRefetchQueries
            : [],
          root,
        );
        const retainedQuery = {
          normalizationAst: normalizationAst.selections,
          variables,
          root,
        };
        status = {
          kind: 'UndisposedComplete',
          retainedQuery,
        };
        retainQuery(environment, retainedQuery);
      }

      const onComplete = fetchOptions?.onComplete;
      if (onComplete != null) {
        let data = readDataForOnComplete(
          artifact,
          environment,
          root,
          variables,
        );

        try {
          // @ts-expect-error this problem will be fixed when we remove RefetchQueryNormalizationArtifact
          // (or we can fix this by having a single param of type { kind: 'Entrypoint', entrypoint,
          // fetchOptions: FetchOptions<TReadFromStore> } | { kind: 'RefetchQuery', refetchQuery,
          // fetchOptions: FetchOptions<void> }).
          onComplete(data);
        } catch {}
      }
    })
    .catch((e) => {
      logMessage(environment, {
        kind: 'ReceivedNetworkError',
        networkRequestId: myNetworkRequestId,
        error: e,
      });
      try {
        fetchOptions?.onError?.();
      } catch {}
      throw e;
    });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<void, AnyError>> = [
    wrapper,
    () => {
      if (status.kind === 'UndisposedComplete') {
        const didUnretainSomeQuery = unretainQuery(
          environment,
          status.retainedQuery,
        );
        if (didUnretainSomeQuery) {
          garbageCollectEnvironment(environment);
        }
      }
      status = {
        kind: 'Disposed',
      };
    },
  ];
  return response;
}

type NetworkRequestStatus =
  | {
      readonly kind: 'UndisposedIncomplete';
    }
  | {
      readonly kind: 'Disposed';
    }
  | {
      readonly kind: 'UndisposedComplete';
      readonly retainedQuery: RetainedQuery;
    };

function readDataForOnComplete<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  environment: IsographEnvironment,
  root: Link,
  variables: ExtractParameters<TReadFromStore>,
): TClientFieldValue | null {
  // An entrypoint, but not a RefetchQueryNormalizationArtifact, has a reader ASTs.
  // So, we can only pass data to onComplete if makeNetworkRequest was passed an entrypoint.
  // This is awkward, since we don't express that in the types of the parameters
  // (i.e. FetchOptions could be passed, along with a RefetchQueryNormalizationArtifact).
  //
  // However, this isn't a big deal: RefetchQueryNormalizationArtifact is going away.
  if (artifact.kind === 'Entrypoint') {
    // TODO this is a smell!
    const fakeNetworkRequest = wrapResolvedValue(undefined);
    // TODO this is a smell — we know the network response is not in flight,
    // so we don't really care!
    const fakeNetworkRequestOptions = {
      suspendIfInFlight: false,
      throwOnNetworkError: false,
    };

    const fragmentResult = readButDoNotEvaluate(
      environment,
      {
        kind: 'FragmentReference',
        // TODO this smells.
        readerWithRefetchQueries: wrapResolvedValue(
          artifact.readerWithRefetchQueries,
        ),
        root,
        variables,
        networkRequest: fakeNetworkRequest,
      },
      fakeNetworkRequestOptions,
    ).item;
    const readerArtifact = artifact.readerWithRefetchQueries.readerArtifact;
    switch (readerArtifact.kind) {
      case 'ComponentReaderArtifact': {
        // @ts-expect-error We should find a way to encode this in the type system:
        // if we have a ComponentReaderArtifact, we will necessarily have a
        // TClientFieldValue which is a React.FC<...>
        return getOrCreateCachedComponent(
          environment,
          readerArtifact.componentName,
          {
            kind: 'FragmentReference',
            readerWithRefetchQueries: wrapResolvedValue({
              kind: 'ReaderWithRefetchQueries',
              readerArtifact: readerArtifact,
              nestedRefetchQueries:
                artifact.readerWithRefetchQueries.nestedRefetchQueries,
            }),
            root,
            variables,
            networkRequest: fakeNetworkRequest,
          } as const,
          fakeNetworkRequestOptions,
        );
      }
      case 'EagerReaderArtifact': {
        return readerArtifact.resolver({
          data: fragmentResult,
          parameters: variables,
        });
      }
      default: {
        const _: never = readerArtifact;
        _;
        throw new Error('Expected case');
      }
    }
  }
  return null;
}
