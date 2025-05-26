# Crux Development Process

This document describes the development process for Crux. It is intended for
anyone considering opening an **issue** or **pull request**. If in doubt,
please open a [Zulip chat topic](https://crux-community.zulipchat.com/) or [discussion](https://github.com/redbadger/crux/discussions);
we can always convert that to an issue later.

Most of the communication for this project happens in Zulip.

## Quick Guide

**I'd like to contribute!**

All issues are actionable. Pick one and start working on it. Thank you.
If you need help or guidance, comment on the issue. Issues that are extra
friendly to new contributors are tagged with "contributor friendly".

**I have a bug!**

1. Search the issue tracker and discussions for similar issues.
2. If you don't have steps to reproduce, open a chat topic or discussion.
3. If you have steps to reproduce, open an issue.

**I have an idea for a feature!**

1. Open a chat topic.

**I've implemented a feature!**

1. If there is an issue for the feature, open a pull request.
2. If there is no issue, open a chat topic and link to your branch.
3. If you want to live dangerously, open a pull request and hope for the best.

**I have a question!**

1. Open a chat topic.

## General Patterns

### Issues are Actionable

The Crux [issue tracker](https://github.com/redbadger/crux/issues)
is for _actionable items_.

Unlike some other projects, Crux **does not use the issue tracker for
discussion or feature requests**. Instead, we
use [Zulip](https://crux-community.zulipchat.com/) and
GitHub [discussions](https://github.com/redbadger/crux/discussions) for that.
Once a discussion reaches a point where a well-understood, actionable
item is identified, it is moved to the issue tracker. **This pattern
makes it easier for maintainers or contributors to find issues to work on
since _every issue_ is ready to be worked on.**

If you are experiencing a bug and have clear steps to reproduce it, please
open an issue. If you are experiencing a bug but you are not sure how to
reproduce it or aren't sure if it's a bug, please open a discussion.
If you have an idea for a feature, please open a discussion.

### Pull Requests Implement an Issue

Pull requests should be associated with a previously accepted issue.
**If you open a pull request for something that wasn't previously discussed,**
it may be closed or remain stale for an indefinite period of time. We're not
saying it will never be accepted, but the odds are stacked against you.

Issues tagged with "feature" represent accepted, well-scoped feature requests.
If you implement an issue tagged with feature as described in the issue, your
pull request will be accepted with a high degree of certainty.

> [!NOTE]
>
> **Pull requests are NOT a place to discuss feature design.** Please do
> not open a WIP pull request to discuss a feature. Instead, use a discussion
> and link to your branch.
