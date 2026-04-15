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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPear.js
var require_faPear = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPear.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pear";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e20c";
    var svgPathData = "M505 7c9.4 9.4 9.4 24.6 0 33.9L457.9 88.1c14 20.5 22.1 45.3 22.1 71.9c0 30.7-10.8 58.9-28.9 81c-29.9 36.5-61.4 81.8-73.3 127.4c-8.3 32-25 62.3-50.1 87.3c-75 75-196.5 75-271.5 0s-75-196.5 0-271.5c25.1-25.1 55.3-41.8 87.3-50.1c45.7-11.8 90.9-43.4 127.4-73.3c22.1-18 50.3-28.9 81-28.9c26.7 0 51.4 8.2 71.9 22.1L471 7c9.4-9.4 24.6-9.4 33.9 0zM320 320c0-8.8-7.2-16-16-16s-16 7.2-16 16c0 53-43 96-96 96c-8.8 0-16 7.2-16 16s7.2 16 16 16c70.7 0 128-57.3 128-128z";
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
    exports.faPear = exports.definition;
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

// src/_entries/faPear.js
var import_faPear = __toESM(require_faPear());
var export_faPear = import_faPear.definition;
export {
  export_faPear as faPear
};
