## Linked Issues

<!-- Please provide the issue number corresponding to this PR. -->

* Closes #

## Notes to Reviewer

<!--
The following are examples of particular points that you would like reviewers to pay attention to. Add or remove
items as appropriate for this PR.
-->

As a reviewer, please pay particular attention to the following areas when reviewing this PR and tick the above boxes after you have completed the steps.

#### Config Changes
* [ ] Ensure proper order in which CLI and config arguments are applied.
* [ ] Ensure that config changes work with individual build features by running `cargo ci-check-features`.

#### API Changes
* [ ] Test the API endpoints which were added/changed.
* [ ] Ensure that the API response times scale with database size appropriately.
* [ ] Review the API documentation changes and confirm that it matches the actual functionality.
* [ ] Check for breaking changes in the API and matching (conventional) commit message prefix.

#### Test cases
* [ ] Review and run tests that were added/changed.
* [ ] Suggest places that may benefit from test cases.

#### INX Changes
* [ ] Run chronicle using an INX connection.

#### Database Changes
* [ ] Review database queries for correctness/conciseness.
* [ ] Ensure queries are supported by indexes if needed.
* [ ] Check for breaking changes in the data model and matching (conventional) commit message prefix.
