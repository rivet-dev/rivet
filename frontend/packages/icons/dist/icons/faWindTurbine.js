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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWindTurbine.js
var require_faWindTurbine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWindTurbine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wind-turbine";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f89b";
    var svgPathData = "M315.7 414.8c9.1 14.8 32 6.5 29.5-10.7L316.9 212c-.5-3.4 .1-6.9 1.8-9.9L411 31.2c8.3-15.3-10.4-31-24-20.1L234.7 131.7c-2.7 2.1-6 3.4-9.5 3.5L31.1 140.6C13.7 141 9.5 165 25.7 171.4l180.6 71.5c3.2 1.3 5.9 3.5 7.7 6.5L315.7 414.8zM224 326.8L224 464l-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l208 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0 0-33.2-64-104zM256 168a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faWindTurbine = exports.definition;
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

// src/_entries/faWindTurbine.js
var import_faWindTurbine = __toESM(require_faWindTurbine());
var export_faWindTurbine = import_faWindTurbine.definition;
export {
  export_faWindTurbine as faWindTurbine
};
