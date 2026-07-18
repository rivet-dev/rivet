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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBellSchool.js
var require_faBellSchool = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBellSchool.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bell-school";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5d5";
    var svgPathData = "M208 128a80 80 0 1 1 0 160 80 80 0 1 1 0-160zm0 288A208 208 0 1 0 208 0a208 208 0 1 0 0 416zm48-208a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM208 448c-54 0-103.9-17.9-144-48l0 64c0 26.5 21.5 48 48 48l208 0 32 0 40 0c43.4 0 80.1-28.8 92-68.3c16.6-7.6 28-24.3 28-43.7c0-26.5-21.5-48-48-48s-48 21.5-48 48c0 15.5 7.3 29.2 18.7 38c-8 15.4-24.1 26-42.7 26l-40 0 0-64c-40.1 30.1-90 48-144 48z";
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
    exports.faBellSchool = exports.definition;
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

// src/_entries/faBellSchool.js
var import_faBellSchool = __toESM(require_faBellSchool());
var export_faBellSchool = import_faBellSchool.definition;
export {
  export_faBellSchool as faBellSchool
};
