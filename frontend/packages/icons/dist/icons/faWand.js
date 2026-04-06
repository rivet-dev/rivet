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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWand.js
var require_faWand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wand";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f72a";
    var svgPathData = "M497 7c-9-9-23.5-9.4-33-.9L189.3 251.9c-2.9 2.6-6.7 4.1-10.7 4.1L144 256c-8.8 0-16 7.2-16 16l0 27.6c0 4.6-1.9 8.9-5.3 11.9L11.1 411.3C4 417.7 0 426.7 0 436.2c0 8.8 3.5 17.3 9.7 23.5l42.6 42.6c6.2 6.2 14.7 9.8 23.5 9.8c9.5 0 18.5-4 24.8-11.1L372.3 197.3c3-3.4 7.4-5.3 11.9-5.3l15.8 0c8.8 0 16-7.2 16-16l0-21.4c0-3.9 1.5-7.7 4.1-10.7L505.9 48c8.5-9.5 8.1-24-.9-33l-8-8z";
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
    exports.faWand = exports.definition;
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

// src/_entries/faWand.js
var import_faWand = __toESM(require_faWand());
var export_faWand = import_faWand.definition;
export {
  export_faWand as faWand
};
