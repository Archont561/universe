Feature: The Airlock consumer on a host that cannot install anything

  Universe Airlock is the side of the product that runs after the network is
  gone. It reports what it is, in the two forms a script and a human each need,
  and every verb it does not implement is refused by name instead of being
  accepted and forgotten.

  Scenario: The version is reported in the form a script reads
    Given the airlock command line "version --json"
    When it is executed
    Then it succeeds with nothing on stderr
    And the report names universe-airlock and the release train version

  Scenario: The version is reported in the form a human reads
    Given the airlock command line "--version"
    When it is executed
    Then it succeeds with nothing on stderr
    And the report is the binary name and version on one line

  Scenario: The consumer lists what it can do
    Given the airlock command line "--help"
    When it is executed
    Then it succeeds with nothing on stderr
    And the help names the version command


  Scenario: A sealed snapshot verifies completely
    Given a published payload
    When it is verified
    Then the report says the payload was verified
    And the report counts one artifact

  Scenario: A missing object is refused by name
    Given a payload with a missing artifact
    When it is verified
    Then the diagnostic names airlock.payload.missing

  Scenario: A corrupt object is refused by name
    Given a payload with a corrupt artifact
    When it is verified
    Then the diagnostic names airlock.payload.digest

  Scenario: An extra object is refused by name
    Given a payload with an extra object
    When it is verified
    Then the diagnostic names airlock.payload.extra

  Scenario: An object addressed under another platform is refused
    Given a payload naming a wrong-platform object
    When it is verified
    Then the diagnostic names airlock.payload.name

  Scenario: The sealed lane refuses an unsigned payload
    Given a published payload
    When it is verified on the sealed lane
    Then the diagnostic names airlock.signature.absent

  Scenario: A structurally valid signed payload is still refused without a trusted key
    Given a signed payload with correct digests
    When it is verified
    Then the diagnostic names airlock.signature.no-key

  Scenario: A signed payload with no trusted key is refused before artifacts are checked
    Given a payload that is signed and missing its artifact
    When it is verified
    Then the diagnostic names airlock.signature.no-key
    And the diagnostic does not name airlock.payload.missing

  Scenario: A producer verb is refused by name
    Given the airlock command line "publish"
    When it is executed
    Then it exits with the usage code
    And the refusal names the argument it does not implement
