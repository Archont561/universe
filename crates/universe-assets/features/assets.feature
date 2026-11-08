Feature: Assets a rendered workflow depends on

  A generated workflow is only as good as the assets baked into the binary that
  rendered it. The portal must be present without a checkout of the portal
  package, and every action the template uses must already be pinned, because a
  consumer cannot fix an unpinned action from their side.

  Scenario: The portal bundle is embedded, not read from the consumer's disk
    Given the embedded portal bundle
    When it is inspected
    Then it holds an index page

  Scenario: Every action the template can emit is pinned to a commit
    Given the actions the template can emit
    When they are inspected
    Then each one names a forty-character commit
    And each one records the release it was pinned from

  Scenario: The template can hold GitHub expressions without being read as a variable
    Given the workflow template
    When its double braces are inspected
    Then every one of them belongs to a GitHub expression
    And the template writes its own variables in angle brackets

  Scenario: The generated workflow installs the Pixi this workspace builds with
    Given the pinned Pixi version
    When the workspace floor is read
    Then the pinned version is the floor without its leading v
