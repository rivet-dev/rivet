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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKnife.js
var require_faKnife = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKnife.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "knife";
    var width = 512;
    var height = 512;
    var aliases = ["utensil-knife"];
    var unicode = "f2e4";
    var svgPathData = "M217.5 249.3l242-242c4.7-4.7 11-7.3 17.6-7.3c11.2 0 21 7.5 23.5 18.4C505.3 38.6 512 71.5 512 96c0 83.3-48.2 130.5-128.9 209.4c-6.5 6.4-13.3 13-20.3 19.9c-12.1 11.9-28.4 18.7-45.3 18.7l-11 0c-17 0-33.3-6.7-45.3-18.7l-14.6-14.6-192 192c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l192-192 16-16 .1-.1z";
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
    exports.faKnife = exports.definition;
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

// src/_entries/faKnife.js
var import_faKnife = __toESM(require_faKnife());
var export_faKnife = import_faKnife.definition;
export {
  export_faKnife as faKnife
};
