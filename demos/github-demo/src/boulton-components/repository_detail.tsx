import { bDeclare } from "@boulton/react";
import type { ResolverParameterType as RepositoryDetailParams } from "./__boulton/Query__repository_detail.boulton";

export const repository_detail = bDeclare<
  RepositoryDetailParams,
  ReturnType<typeof RepositoryDetail>
>`
  Query.repository_detail @component {
    repository(name: $repositoryName, owner: $repositoryOwner,) {
      id,
      nameWithOwner,
      parent {
        repository_link,
        id,
        nameWithOwner,
      },

      pullRequests(last: $first,) {
        pull_request_table,
      },
    },
  }
`(RepositoryDetail);

function RepositoryDetail(props: RepositoryDetailParams) {
  console.log("repo detail", props.data);
  const parent = props.data.repository?.parent;
  return (
    <>
      <h1>{props.data.repository?.nameWithOwner}</h1>
      {parent != null ? (
        <h3>
          <small>Forked from</small>{" "}
          {parent.repository_link({
            setRoute: props.setRoute,
            children: parent.nameWithOwner,
          })}
        </h3>
      ) : null}
      {props.data.repository?.pullRequests.pull_request_table({
        setRoute: props.setRoute,
      })}
      {/* <div>Stargazer count: {props.data.repository?.stargazerCount}</div> */}
    </>
  );
}
