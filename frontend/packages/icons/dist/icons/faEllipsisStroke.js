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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEllipsisStroke.js
var require_faEllipsisStroke = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEllipsisStroke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ellipsis-stroke";
    var width = 512;
    var height = 512;
    var aliases = ["ellipsis-h-alt"];
    var unicode = "f39b";
    var svgPathData = "M416 280a24 24 0 1 1 0-48 24 24 0 1 1 0 48zm0-96a72 72 0 1 0 0 144 72 72 0 1 0 0-144zM256 280a24 24 0 1 1 0-48 24 24 0 1 1 0 48zm0-96a72 72 0 1 0 0 144 72 72 0 1 0 0-144zM120 256a24 24 0 1 1 -48 0 24 24 0 1 1 48 0zm-96 0a72 72 0 1 0 144 0A72 72 0 1 0 24 256z";
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
    exports.faEllipsisStroke = exports.definition;
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

// src/_entries/faEllipsisStroke.js
var import_faEllipsisStroke = __toESM(require_faEllipsisStroke());
var export_faEllipsisStroke = import_faEllipsisStroke.definition;
export {
  export_faEllipsisStroke as faEllipsisStroke
};
