var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBellSchoolSlash.js
var require_faBellSchoolSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBellSchoolSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bell-school-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5d6";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L570.7 422c3.4-6.6 5.3-14.1 5.3-22c0-26.5-21.5-48-48-48c-12.1 0-23.1 4.5-31.5 11.8L445 323.5c22.1-33 35-72.8 35-115.5C480 93.1 386.9 0 272 0C211.2 0 156.6 26.1 118.5 67.6L38.8 5.1zM220 147.2c14-11.9 32.1-19.2 52-19.2c44.2 0 80 35.8 80 80c0 12.9-3.1 25.1-8.5 35.9l-26.1-20.4c1.7-4.9 2.6-10.1 2.6-15.5c0-26.5-21.5-48-48-48c-9.5 0-18.4 2.8-25.9 7.6L220 147.2zM508.3 496.5L465.8 463c-3.2 .7-6.4 1-9.8 1l-40 0 0-40.2-16.1-12.7C362.9 434.5 319 448 272 448c-54 0-103.9-17.9-144-48l0 64c0 26.5 21.5 48 48 48l208 0 32 0 40 0c19.3 0 37.3-5.7 52.3-15.5zM373 389.9L71.5 152.4C66.6 170.1 64 188.7 64 208c0 114.9 93.1 208 208 208c36.7 0 71.1-9.5 101-26.1z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBellSchoolSlash = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBellSchoolSlash.js
var import_faBellSchoolSlash = __toESM(require_faBellSchoolSlash());
var export_faBellSchoolSlash = import_faBellSchoolSlash.definition;
export {
  export_faBellSchoolSlash as faBellSchoolSlash
};
