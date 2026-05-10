import os

tmpl = open(".github/homebrew/vhsify.rb.tmpl").read()
result = (
    tmpl.replace("{{VERSION}}", os.environ["VERSION"])
        .replace("{{SHA_ARM}}", os.environ["SHA_ARM"])
        .replace("{{SHA_LINUX}}", os.environ["SHA_LINUX"])
)
open("homebrew-tap/Formula/vhsify.rb", "w").write(result)
