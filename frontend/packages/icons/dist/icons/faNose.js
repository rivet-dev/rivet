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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNose.js
var require_faNose = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNose.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "nose";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5bd";
    var svgPathData = "M324.2 231.9L421 338.3c17.4 19.1 27 44 27 69.9C448 465.5 401.5 512 344.1 512l-9.4 0c-32.3 0-63.6-11.3-88.4-32l-6.4-5.3c-20.7-17.2-46.7-26.7-73.6-26.7c-21.2 0-38.3 17.2-38.3 38.3l0 5.7c0 1.7 .1 3.4 .3 5.1c.8 7.1-4 14.9-11.2 14.9l-5.1 0C50.8 512 1.1 462.9 0 401.9C1 345.3 44.3 298 100.9 292.4l44.7-4.5c8.8-.9 15.2-8.7 14.3-17.5s-8.7-15.2-17.5-14.3l-44.7 4.5c-7.1 .7-14 1.9-20.7 3.6c-6.4 1.6-13-3-13-9.7L64 32C64 14.3 78.3 0 96 0l61.6 0c11.4 0 21.9 6 27.6 15.9l75.6 129.6c18.1 30.9 39.3 59.9 63.4 86.4z";
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
    exports.faNose = exports.definition;
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

// src/_entries/faNose.js
var import_faNose = __toESM(require_faNose());
var export_faNose = import_faNose.definition;
export {
  export_faNose as faNose
};
