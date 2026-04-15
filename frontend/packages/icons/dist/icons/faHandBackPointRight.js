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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointRight.js
var require_faHandBackPointRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-back-point-right";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1a1";
    var svgPathData = "M512 168c0-22.1-17.9-40-40-40l-200 0-7 0-73 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l55.7 0c-1.4-2-3-3.9-4.5-5.8L227.2 71C215 56.4 197 48 178 48L140 48c-38 0-74 16.9-98.3 46.1l-4.5 5.4C13.1 128.3 0 164.5 0 201.9L0 320c0 70.7 57.3 128 128 128l64 0 32 0c26.5 0 48-21.5 48-48c0-12.4-4.7-23.6-12.3-32.1C284.5 366 304 345.3 304 320c0-12.4-4.7-23.6-12.3-32.1C316.5 286 336 265.3 336 240c0-12.3-4.6-23.5-12.2-32L472 208c22.1 0 40-17.9 40-40z";
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
    exports.faHandBackPointRight = exports.definition;
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

// src/_entries/faHandBackPointRight.js
var import_faHandBackPointRight = __toESM(require_faHandBackPointRight());
var export_faHandBackPointRight = import_faHandBackPointRight.definition;
export {
  export_faHandBackPointRight as faHandBackPointRight
};
