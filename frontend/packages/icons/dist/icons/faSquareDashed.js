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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDashed.js
var require_faSquareDashed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDashed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-dashed";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e269";
    var svgPathData = "M96 32l32 0 0 64L96 96c-17.7 0-32 14.3-32 32l0 32L0 160l0-32C0 75 43 32 96 32zM0 192l64 0 0 128L0 320 0 192zm384 0l64 0 0 128-64 0 0-128zm64-32l-64 0 0-32c0-17.7-14.3-32-32-32l-32 0 0-64 32 0c53 0 96 43 96 96l0 32zm0 192l0 32c0 53-43 96-96 96l-32 0 0-64 32 0c17.7 0 32-14.3 32-32l0-32 64 0zM64 352l0 32c0 17.7 14.3 32 32 32l32 0 0 64-32 0c-53 0-96-43-96-96l0-32 64 0zm96 128l0-64 128 0 0 64-128 0zm0-384l0-64 128 0 0 64L160 96z";
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
    exports.faSquareDashed = exports.definition;
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

// src/_entries/faSquareDashed.js
var import_faSquareDashed = __toESM(require_faSquareDashed());
var export_faSquareDashed = import_faSquareDashed.definition;
export {
  export_faSquareDashed as faSquareDashed
};
