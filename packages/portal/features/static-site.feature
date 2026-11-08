Feature: The portal is a file tree a static host can serve

  The portal is built once and served from a bucket, so anything it needs at request time is
  something the host cannot give it. Every route is therefore rendered at build time, every URL
  is addressed the one way a static host resolves it, and the page says which version of the
  package it was built from.

  Scenario: Every route is rendered at build time
    Given the portal's route contract
    When it is read
    Then prerendering is enabled for every route

  Scenario: Every route is addressed with a trailing slash
    Given the portal's route contract
    When it is read
    Then a trailing slash is the canonical form

  Scenario: The landing page shows the version the package declares
    Given the landing page source
    When it is inspected
    Then it reads the version from the portal library
    And it shows the version in a badge a test can find
