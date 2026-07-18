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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faJug.js
var require_faJug = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faJug.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "jug";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8c6";
    var svgPathData = "M160 0L288 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L160 64c-17.7 0-32-14.3-32-32s14.3-32 32-32zM400 168c0-13.3-10.7-24-24-24c-10.3 0-19.1 6.5-22.5 15.6l25.1 32.2c12.1-1.3 21.4-11.5 21.4-23.9zm-89.3-63.3l11.8 15.1C335.7 105.2 354.8 96 376 96c39.8 0 72 32.2 72 72c0 29.2-17.4 54.3-42.3 65.6c6.8 15.8 10.3 32.9 10.3 50.3L416 448c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-164.1c0-28.5 9.5-56.1 27-78.6l78.3-100.7c2.1-2.6 3.7-5.6 4.8-8.7l163.9 0c1.1 3.1 2.7 6 4.8 8.7zM128 288c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l192 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-192 0z";
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
    exports.faJug = exports.definition;
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

// src/_entries/faJug.js
var import_faJug = __toESM(require_faJug());
var export_faJug = import_faJug.definition;
export {
  export_faJug as faJug
};
