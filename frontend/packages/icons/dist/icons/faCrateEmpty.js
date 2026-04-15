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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCrateEmpty.js
var require_faCrateEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCrateEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crate-empty";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e151";
    var svgPathData = "M48 224c-26.5 0-48 21.5-48 48l0 80 512 0 0-80c0-26.5-21.5-48-48-48L48 224zM512 384L0 384l0 80c0 26.5 21.5 48 48 48l416 0c26.5 0 48-21.5 48-48l0-80zM64 272a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM48 448a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM448 272a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM432 448a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faCrateEmpty = exports.definition;
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

// src/_entries/faCrateEmpty.js
var import_faCrateEmpty = __toESM(require_faCrateEmpty());
var export_faCrateEmpty = import_faCrateEmpty.definition;
export {
  export_faCrateEmpty as faCrateEmpty
};
