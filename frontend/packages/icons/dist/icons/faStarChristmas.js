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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarChristmas.js
var require_faStarChristmas = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarChristmas.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "star-christmas";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7d4";
    var svgPathData = "M15.5 275.9L192 320l44.1 176.5c2.3 9.1 10.5 15.5 19.9 15.5s17.6-6.4 19.9-15.5L320 320l176.5-44.1c9.1-2.3 15.5-10.5 15.5-19.9s-6.4-17.6-15.5-19.9L320 192 275.9 15.5C273.6 6.4 265.4 0 256 0s-17.6 6.4-19.9 15.5L192 192 15.5 236.1C6.4 238.4 0 246.6 0 256s6.4 17.6 15.5 19.9zM71 71c-9.4 9.4-9.4 24.6 0 33.9l48 48c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L105 71c-9.4-9.4-24.6-9.4-33.9 0zM441 71c-9.4-9.4-24.6-9.4-33.9 0l-48 48c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l48-48c9.4-9.4 9.4-24.6 0-33.9zm0 336l-48-48c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l48 48c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9zM71 441c9.4 9.4 24.6 9.4 33.9 0l48-48c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L71 407c-9.4 9.4-9.4 24.6 0 33.9z";
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
    exports.faStarChristmas = exports.definition;
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

// src/_entries/faStarChristmas.js
var import_faStarChristmas = __toESM(require_faStarChristmas());
var export_faStarChristmas = import_faStarChristmas.definition;
export {
  export_faStarChristmas as faStarChristmas
};
