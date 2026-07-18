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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleSterling.js
var require_faCircleSterling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleSterling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-sterling";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5cf";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm5.5-352c-16.8 0-30.4 13.7-30.3 30.5l.2 33.5 40.5 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-40.3 0 .1 9.7c.1 18.6-3.1 37-9.6 54.3L328 320c13.3 0 24 10.7 24 24s-10.7 24-24 24l-144 0c-8.4 0-16.1-4.4-20.5-11.5s-4.7-16-.8-23.5l9.6-18.6c7.7-14.9 11.6-31.5 11.5-48.3l-.1-10.1-7.7 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l7.5 0-.2-33.1c-.3-43.5 34.8-78.9 78.3-78.9c10 0 19.8 1.9 29.1 5.6l30.3 12.1c12.3 4.9 18.3 18.9 13.4 31.2s-18.9 18.3-31.2 13.4l-30.3-12.1c-3.6-1.4-7.4-2.2-11.3-2.2z";
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
    exports.faCircleSterling = exports.definition;
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

// src/_entries/faCircleSterling.js
var import_faCircleSterling = __toESM(require_faCircleSterling());
var export_faCircleSterling = import_faCircleSterling.definition;
export {
  export_faCircleSterling as faCircleSterling
};
