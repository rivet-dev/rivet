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

// src/node_modules/@fortawesome/free-brands-svg-icons/fa42Group.js
var require_fa42Group = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/fa42Group.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "42-group";
    var width = 640;
    var height = 512;
    var aliases = ["innosoft"];
    var unicode = "e080";
    var svgPathData = "M320 96l0 320c21 0 41.8-4.1 61.2-12.2s37-19.8 51.9-34.7 26.6-32.5 34.7-51.9 12.2-40.2 12.2-61.2-4.1-41.8-12.2-61.2-19.8-37.1-34.7-51.9-32.5-26.6-51.9-34.7-40.2-12.2-61.2-12.2zM0 256L160 416 320 256 160 96 0 256zm480 0c0 21 4.1 41.8 12.2 61.2s19.8 37 34.7 51.9 32.5 26.6 51.9 34.7 40.2 12.2 61.2 12.2l0-320c-42.4 0-83.1 16.9-113.1 46.9S480 213.6 480 256z";
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
    exports.fa42Group = exports.definition;
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

// src/_entries/faInnosoft.js
var import_fa42Group = __toESM(require_fa42Group());
var export_faInnosoft = import_fa42Group.definition;
export {
  export_faInnosoft as faInnosoft
};
