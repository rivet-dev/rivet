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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserTie.js
var require_faUserTie = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserTie.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-tie";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f508";
    var svgPathData = "M224 248a120 120 0 1 1 0-240 120 120 0 1 1 0 240zm-30.5 56l61 0c9.7 0 17.5 7.8 17.5 17.5 0 4.2-1.5 8.2-4.2 11.4l-27.4 32 31 115.1 .6 0 34.6-138.5c2.2-8.7 11.1-14 19.5-10.8 61.9 23.6 105.9 83.6 105.9 153.8 0 15.1-12.3 27.4-27.4 27.4L43.4 512c-15.1 0-27.4-12.3-27.4-27.4 0-70.2 44-130.2 105.9-153.8 8.4-3.2 17.3 2.1 19.5 10.8l34.6 138.5 .6 0 31-115.1-27.4-32c-2.7-3.2-4.2-7.2-4.2-11.4 0-9.7 7.8-17.5 17.5-17.5z";
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
    exports.faUserTie = exports.definition;
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

// src/_entries/faUserTie.js
var import_faUserTie = __toESM(require_faUserTie());
var export_faUserTie = import_faUserTie.definition;
export {
  export_faUserTie as faUserTie
};
