# Wheelie rpm spec. Built by packaging/linux/make-rpm.sh, which stages the
# prebuilt release binary and data files under SOURCES/stage as an install
# tree. Versions come from @VERSION@ (dashes already rewritten to ~).
Name:           wheelie
Version:        @VERSION@
Release:        1
Summary:        Knob utility with a radial wheel UI
License:        GPL-3.0-only
URL:            https://github.com/RedsTom/wheelie
Source0:        stage

%global debug_package %{nil}
AutoReqProv:    no

%description
Turn a keyboard knob into a configurable radial wheel. Wheelie captures
the keys your keyboard software emits for the knob and runs your own
actions: volume, brightness, scroll, macros, shortcuts and more.

%prep
# Nothing to prepare: the binary is staged already built.

%build
# Nothing to build.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a %{S:0}/usr %{buildroot}/usr

%files
%defattr(-,root,root,-)
/usr/bin/wheelie
/usr/share/applications/wheelie.desktop
/usr/share/icons/hicolor/256x256/apps/wheelie.png
/usr/share/icons/hicolor/scalable/apps/wheelie.svg
%doc /usr/share/doc/wheelie/LICENSE

%changelog
* Thu Jan 01 2026 RedsTom - @VERSION@-1
- Release built by the GitHub release workflow.
