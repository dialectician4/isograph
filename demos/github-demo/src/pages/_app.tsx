import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';
import type { AppProps } from 'next/app';
import { useMemo } from 'react';

function makeNetworkRequest<T>(
  queryText: string,
  variables: unknown,
): Promise<T> {
  const promise = fetch('https://api.github.com/graphql', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + process.env.NEXT_PUBLIC_GITHUB_TOKEN,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then(async (response) => {
    const json = await response.json();

    if (response.ok) {
      /**
       * Enforce that the network response follows the specification:: {@link https://spec.graphql.org/draft/#sec-Errors}.
       */
      if (Object.hasOwn(json, 'errors')) {
        if (!Array.isArray(json.errors) || json.errors.length === 0) {
          throw new Error('GraphQLSpecificationViolationError', {
            cause: json,
          });
        }
        throw new Error('GraphQLError', {
          cause: json.errors,
        });
      }
      return json;
    }
    throw new Error('NetworkError', {
      cause: json,
    });
  });
  return promise;
}

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(() => {
    return createIsographEnvironment(
      createIsographStore(),
      makeNetworkRequest,
      null,
      typeof window != 'undefined' ? console.log : null,
    );
  }, []);
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Component {...pageProps} />
    </IsographEnvironmentProvider>
  );
}
