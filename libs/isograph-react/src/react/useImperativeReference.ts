import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { ROOT_ID } from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

// TODO rename this to useImperativelyLoadedEntrypoint
type UseImperativeReferenceResult<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
> = {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    ...[fetchOptions]: NormalizationAstLoader extends TNormalizationAst
      ? [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
      : [fetchOptions?: FetchOptions<TClientFieldValue>]
  ) => void;
};

export function useImperativeReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst
  >,
): UseImperativeReferenceResult<
  TReadFromStore,
  TClientFieldValue,
  TNormalizationAst
> {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state,
    loadFragmentReference: (
      variables: ExtractParameters<TReadFromStore>,
      fetchOptions?: FetchOptions<TClientFieldValue>,
    ) => {
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
        environment,
        entrypoint as IsographEntrypoint<any, any, NormalizationAst>,
        variables,
        fetchOptions,
      );
      setState([
        {
          kind: 'FragmentReference',
          readerWithRefetchQueries: wrapResolvedValue({
            kind: 'ReaderWithRefetchQueries',
            readerArtifact: entrypoint.readerWithRefetchQueries.readerArtifact,
            nestedRefetchQueries:
              entrypoint.readerWithRefetchQueries.nestedRefetchQueries,
          }),
          root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
          variables,
          networkRequest,
        },
        () => {
          disposeNetworkRequest();
        },
      ]);
    },
  };
}
