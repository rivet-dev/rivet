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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSquareRootVariable.js
var require_faSquareRootVariable = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSquareRootVariable.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-root-variable";
    var width = 576;
    var height = 512;
    var aliases = ["square-root-alt"];
    var unicode = "f698";
    var svgPathData = "M282.6 78.1c8-27.3 33-46.1 61.4-46.1l200 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L344 96 238.7 457c-3.6 12.3-14.1 21.2-26.8 22.8s-25.1-4.6-31.5-15.6L77.6 288 32 288c-17.7 0-32-14.3-32-32s14.3-32 32-32l45.6 0c22.8 0 43.8 12.1 55.3 31.8l65.2 111.8 84.4-289.5zM393.4 233.4c12.5-12.5 32.8-12.5 45.3 0l41.4 41.4 41.4-41.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-41.4 41.4 41.4 41.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-41.4-41.4-41.4 41.4c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l41.4-41.4-41.4-41.4c-12.5-12.5-12.5-32.8 0-45.3z";
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
    exports.faSquareRootVariable = exports.definition;
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

// src/_entries/faSquareRootVariable.js
var import_faSquareRootVariable = __toESM(require_faSquareRootVariable());
var export_faSquareRootVariable = import_faSquareRootVariable.definition;
export {
  export_faSquareRootVariable as faSquareRootVariable
};
