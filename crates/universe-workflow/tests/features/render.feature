Feature: Rendering the workflow set from a project manifest
  A project states what it builds and where its bytes go. The renderer turns that
  statement into the workflow files that do the publishing, so the manifest stays
  the only thing a maintainer edits and a generated file is never the source of
  truth.

  Scenario: Two writable stores produce one publish job each
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then the publish file has a job for the hosted channel
    And the publish file has a job for the static channel
    And the publish file prints the store to package mapping

  Scenario: A project that publishes nothing renders only the main file
    Given a project that publishes nothing
    When the workflow set is rendered
    Then the rendered file set is the main workflow file alone

  Scenario: A hand-edited generated file is reported as drift
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    And the main workflow file is edited by hand
    Then the drift check names the main workflow file

  Scenario: A Conda package cannot be published to an npm store
    Given a project whose Conda package publishes to an npm store
    When the workflow set is rendered
    Then rendering is refused because the store kind does not accept the package kind

  Scenario: Rendering the same manifest twice produces the same bytes
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    And the same manifest is rendered again
    Then both renders are byte-identical

  Scenario: Every job gets a runner, and one that runs a project command checks out first
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then every job names the runner it runs on
    And every job that runs a project command checks the project out and sets up Pixi first
