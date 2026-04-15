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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgeSheriff.js
var require_faBadgeSheriff = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgeSheriff.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "badge-sheriff";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8a2";
    var svgPathData = "M272 48c0 14.8-6.7 28-17.2 36.8l25 42.8c5.7 9.8 16.2 15.8 27.5 15.9l47.6 .2c6.7-18.5 24.4-31.7 45.2-31.7c26.5 0 48 21.5 48 48s-21.5 48-48 48c-5.9 0-11.6-1.1-16.9-3.1L363 240.1c-5.6 9.8-5.6 21.9 0 31.8l20.1 35.2c5.3-2 10.9-3.1 16.9-3.1c26.5 0 48 21.5 48 48s-21.5 48-48 48c-20.8 0-38.5-13.2-45.2-31.7l-47.6 .2c-11.3 0-21.8 6.1-27.5 15.9l-25 42.8c10.5 8.8 17.2 22 17.2 36.8c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-14.8 6.7-28 17.2-36.8l-25-42.8c-5.7-9.8-16.2-15.8-27.5-15.9l-47.6-.2C86.5 386.8 68.8 400 48 400c-26.5 0-48-21.5-48-48s21.5-48 48-48c5.9 0 11.6 1.1 16.9 3.1L85 271.9c5.6-9.8 5.6-21.9 0-31.8L64.9 204.9c-5.3 2-10.9 3.1-16.9 3.1c-26.5 0-48-21.5-48-48s21.5-48 48-48c20.8 0 38.5 13.2 45.2 31.7l47.6-.2c11.3 0 21.8-6.1 27.5-15.9l25-42.8C182.7 76 176 62.8 176 48c0-26.5 21.5-48 48-48s48 21.5 48 48zM224 320a64 64 0 1 0 0-128 64 64 0 1 0 0 128z";
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
    exports.faBadgeSheriff = exports.definition;
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

// src/_entries/faBadgeSheriff.js
var import_faBadgeSheriff = __toESM(require_faBadgeSheriff());
var export_faBadgeSheriff = import_faBadgeSheriff.definition;
export {
  export_faBadgeSheriff as faBadgeSheriff
};
