import time
import re

from pyre.helpers import parse_route, PathNotLastError
from pyre.helpers.route_mapper import _standard_type_converter


def test_regex(regex_str: str, test_string: str):
    regex = re.compile(regex_str, re.VERBOSE)
    start = time.perf_counter()
    x = regex.fullmatch(test_string) is not None
    stop = time.perf_counter() - start
    return x, stop * 100


def make_route_str(combination: tuple, seperator: str):
    route_str = "/"
    for letter, com in zip("abcdefghijklmnopqrstuv", combination):
        route_str += f"{seperator}/"
        route_str += f"{{{letter}:{com}}}/"
    return route_str


def make_pass_string(combination: tuple, seperator: str):
    good_converter = {
        "alpha": r"foo",
        "alnum": r"f00",
        "string": r"h3llo-w0rld",
        "int": r"13058",
        "path": r"world/hello.txt",
        "uuid": r"6a2f41a3-c54c-fce8-32d2-0324e1c32e22"
    }

    good = "/"
    for com in combination:
        success_str = good_converter[com]
        good += f"{seperator}/{success_str}/"
    return good


def make_fail_string(combination: tuple, seperator: str):
    bad_converter = {
        "alpha": r"f0o",
        "alnum": r"f00-bbsf",
        "string": r"h3l/lo-/w0rld",
        "int": r"13A58",
        "path": r"world/hello.txt",
        "uuid": r"6a2f41afa3-c54c-fsfce8-32d2-0324ea1c32e22"
    }

    bad = "/"
    for com in combination:
        success_str = bad_converter[com]
        bad += f"{seperator}/{success_str}/"
    return bad


if __name__ == '__main__':
    import itertools

    combinations = itertools.permutations(_standard_type_converter.keys(), len(_standard_type_converter.keys()))

    passed = True
    times, count = [], 0
    for i, combo in enumerate(combinations):
        route = make_route_str(combo, "abc")
        pass_str = make_pass_string(combo, "abc")
        fail_str = make_fail_string(combo, "abc")

        try:
            built_regex = parse_route(route)
        except PathNotLastError:
            if combo.index("path") == (len(combo) - 1):
                print(f"Test Failed! - Path should have succeeded.\n"
                      f" Combo: {combo!r}\n"
                      f" Route: {route!r}\n"
                      f" ")
                passed = False
                break
            continue

        matched, timed = test_regex(built_regex, pass_str)
        if not matched:
            print(f"Test Failed! - Route should have matched\n"
                  f" Combo: {combo!r}\n"
                  f" Route: {route!r}\n"
                  f" Built Route: {built_regex!r}\n"
                  f" Pass Str: {pass_str!r}\n")
            passed = False
            break
        times.append(timed)

        matched, timed = test_regex(built_regex, pass_str)
        if not matched:
            print(f"Test Failed! - Route should have failed to match\n"
                  f" Combo: {combo!r}\n"
                  f" Route: {route!r}\n"
                  f" Built Route: {built_regex!r}\n"
                  f" Fail Str: {fail_str!r}\n")
            passed = False
            break
        times.append(timed)
        count = i + 1

    if passed:
        print(f"All Test OK! - Ran {count} tests.\n"
              f" Avg Latency: {(sum(times) / len(times))*1000}ms\n")
    else:
        print(f"Test failed on iteration: {count}")