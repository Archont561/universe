Feature: Establishing Airlock on a host that has no network

  The bootstrap is the only thing a disconnected host runs. It reads a payload it was
  handed, writes nothing unless it can verify what it read, and never replaces a working
  environment with one it could not verify. The developer lane is the one that accepts an
  unsigned payload, and it says so; the sealed lane refuses anything it cannot verify.

  Scenario: Verification reads a payload and installs nothing
    Given a payload published for the pinned commit
    When the payload is verified
    Then verification succeeds and names the release version
    And it says nothing was activated
    And the state root is still empty

  Scenario: A restore materialises the payload and makes it current
    Given a payload whose artifact matches its manifest
    When the pinned commit is restored
    Then the restore succeeds and names the release version
    And the artifact sits under the environment current names

  Scenario: A restore of a commit the payload does not name is refused
    Given a payload that names only the pinned commit
    When a different commit is restored
    Then the restore is refused because the commit does not match
    And nothing became current

  Scenario: A sealed restore refuses an unsigned payload
    Given an unsigned payload published for the pinned commit
    When the payload is restored in the sealed lane
    Then the restore is refused because the signature is absent
    And nothing became current

  Scenario: A second restore reuses the environment it already made
    Given a payload that has already been restored once
    When the pinned commit is restored again
    Then the second restore succeeds without copying again
