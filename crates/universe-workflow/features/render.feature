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

  Scenario: The hosted channel is reached with a token minted from the job's identity
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then the hosted job may mint an identity token
    And the hosted job exports no long-lived secret

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

  Scenario: Every Universe command runs through the Pixi task that built it
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then no step runs a bare universe command

  Scenario: A build's artifacts reach the jobs that audit and publish them
    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then the build job uploads the artifacts it produced
    And every job that audits, verifies, or publishes downloads them

  Scenario: A hosted channel names the GitHub environment its publisher expects
    A trusted publisher declared with an environment only accepts a token whose
    `environment` claim names it, and GitHub adds that claim to a job that declares
    the environment. The name belongs to the store, not to every job in the file.

    Given a project whose hosted channel names a GitHub environment
    When the workflow set is rendered
    Then only the hosted publish job declares that environment

  Scenario: A publish build emits the fragments its publishers read
    A publisher refuses an artifact with no `output/fragments`, so the publish file
    cannot leave auditing to a different workflow run. The fragments have to be
    inside the tree the build uploads, not merely produced somewhere on a runner.

    Given a project that publishes its Airlock package to a hosted channel and to a static channel
    When the workflow set is rendered
    Then the publish build writes fragments into the artifact before uploading it

  Scenario: Lock adapters add the Airlock file with a job per declared format
    Given a project that declares Pixi and Bun lock adapters
    When the workflow set is rendered
    Then the rendered file set includes the Airlock file
    And the Airlock file exercises every declared lock format

  Scenario: A project that verifies published destinations gets a separate reader job
    Given a project that verifies every published destination
    When the workflow set is rendered
    Then the publish file holds a separate reader for the anonymous destination

  Scenario: A released package reaches GitHub Releases without naming the store
    `releases` is the one implicit destination. A package with `release = true`
    does not list it, but the renderer still gives it a publish job that creates
    a GitHub Release asset.

    Given a project whose package is released implicitly
    When the workflow set is rendered
    Then the publish file carries a releases job for that package
