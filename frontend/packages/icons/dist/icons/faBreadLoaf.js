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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBreadLoaf.js
var require_faBreadLoaf = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBreadLoaf.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bread-loaf";
    var width = 640;
    var height = 512;
    var aliases = [127838];
    var unicode = "f7eb";
    var svgPathData = "M0 192C0 64 176 32 240 32s240 32 240 160c0 35.3-28.7 64-64 64l0 176c0 26.5-21.5 48-48 48l-256 0c-26.5 0-48-21.5-48-48l0-176c-35.3 0-64-28.7-64-64zM397.4 34.9c-2-.9-4.1-1.9-6.2-2.8c3.1-.1 6-.2 8.8-.2c64 0 240 32 240 160c0 35.3-28.7 64-64 64l0 176c0 26.5-21.5 48-48 48l-96 0c10-13.4 16-30 16-48l0-149.5c37.3-13.2 64-48.7 64-90.5c0-82.2-57.6-130.7-114.6-157.1z";
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
    exports.faBreadLoaf = exports.definition;
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

// src/_entries/faBreadLoaf.js
var import_faBreadLoaf = __toESM(require_faBreadLoaf());
var export_faBreadLoaf = import_faBreadLoaf.definition;
export {
  export_faBreadLoaf as faBreadLoaf
};
