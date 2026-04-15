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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBlackberry.js
var require_faBlackberry = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBlackberry.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "blackberry";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f37b";
    var svgPathData = "M166 116.9c0 23.4-16.4 49.1-72.5 49.1l-70.1 0 21-88.8 67.8 0c42.1 0 53.8 23.3 53.8 39.7zM292.2 77.2l-67.8 0-18.7 88.8 70.1 0c53.8 0 70.1-25.7 70.1-49.1 .1-16.4-11.6-39.7-53.7-39.7zM88.8 208.1l-67.8 0-21 88.8 70.1 0c56.1 0 72.5-23.4 72.5-49.1 0-16.3-11.7-39.7-53.8-39.7zm180.1 0l-67.8 0-18.7 88.8 70.1 0c53.8 0 70.1-23.4 70.1-49.1 0-16.3-11.7-39.7-53.7-39.7zm189.3-53.8l-67.8 0-18.7 88.8 70.1 0c53.8 0 70.1-23.4 70.1-49.1 .1-16.3-11.6-39.7-53.7-39.7zm-28 137.9l-67.8 0-18.7 88.8 70.1 0c56.1 0 70.1-23.4 70.1-49.1 0-16.3-11.6-39.7-53.7-39.7zM240.8 346l-67.8 0-18.7 88.8 70.1 0c56.1 0 70.1-25.7 70.1-49.1 .1-16.3-11.6-39.7-53.7-39.7z";
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
    exports.faBlackberry = exports.definition;
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

// src/_entries/faBlackberry.js
var import_faBlackberry = __toESM(require_faBlackberry());
var export_faBlackberry = import_faBlackberry.definition;
export {
  export_faBlackberry as faBlackberry
};
