import { FC } from "react";

const HamburgerIcon = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    fill="none"
    viewBox="0 0 24 24"
    className="inline-block w-6 h-6 stroke-current"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      d="M4 6h16M4 12h16M4 18h16"
    ></path>
  </svg>
);

interface NavbarProps {
  title: String;
}

const Navbar: FC<NavbarProps> = ({ title }) => {
  return (
    <header className="navbar mb-2 shadow-md bg-neutral text-neutral-content">
      <div className="flex-none">
        <button className="btn btn-square btn-ghost">
          <HamburgerIcon />
        </button>
      </div>
      <div className="flex-1 px-2 mx-2">
        <span className="text-lg font-bold">{title}</span>
      </div>
    </header>
  );
};

export default Navbar;
