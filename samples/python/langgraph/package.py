"""Package the LangGraph sample into an OCI-based ADPKG layout."""
from pathlib import Path

from adp_sdk.adpkg import ADPackage


def main() -> None:
    root = Path(__file__).resolve().parent
    out = root / "langgraph-oci"
    out.mkdir(exist_ok=True)
    ADPackage.create_from_directory(root, out)
    print(f"Wrote OCI ADPKG layout to {out}")


if __name__ == "__main__":
    main()
