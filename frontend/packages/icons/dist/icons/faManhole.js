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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faManhole.js
var require_faManhole = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faManhole.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "manhole";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1d6";
    var svgPathData = "M448 256A192 192 0 1 0 64 256a192 192 0 1 0 384 0zM0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM281.4 115.4c5.8-7.5 14.5-14.7 23.6-11.8c64.4 20.7 111 81.1 111 152.4s-46.6 131.7-111 152.4c-9 2.9-17.8-4.3-23.6-11.8c-5.8-7.6-15.1-12.6-25.4-12.6s-19.6 4.9-25.4 12.6c-5.8 7.5-14.5 14.7-23.6 11.8C142.6 387.7 96 327.3 96 256s46.6-131.7 111-152.4c9-2.9 17.8 4.3 23.6 11.8c5.8 7.6 15.1 12.6 25.4 12.6s19.6-4.9 25.4-12.6zM192 176a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm0 64a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm0 64a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm80-112a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm-32 64a16 16 0 1 0 32 0 16 16 0 1 0 -32 0zm32 64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48-144a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm0 64a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm0 64a16 16 0 1 0 0 32 16 16 0 1 0 0-32z";
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
    exports.faManhole = exports.definition;
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

// src/_entries/faManhole.js
var import_faManhole = __toESM(require_faManhole());
var export_faManhole = import_faManhole.definition;
export {
  export_faManhole as faManhole
};
