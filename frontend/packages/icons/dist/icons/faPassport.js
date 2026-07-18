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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPassport.js
var require_faPassport = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPassport.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "passport";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f5ab";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L320 0c35.3 0 64 28.7 64 64l0 384c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 64zM96 408c0 13.3 10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-144 0c-13.3 0-24 10.7-24 24zM278.6 208c-4.8 26.4-21.5 48.7-44.2 61.2 6.7-17 11.2-38 12.6-61.2l31.6 0zm-173.1 0l31.6 0c1.4 23.1 6 44.2 12.6 61.2-22.7-12.5-39.4-34.8-44.2-61.2zm76.4 55c-6.2-13.4-11.1-32.5-12.7-55l45.8 0c-1.6 22.5-6.5 41.6-12.7 55-4.5 9.6-8.2 13.8-10.2 15.5-2-1.7-5.7-5.8-10.2-15.5zm0-142c4.5-9.6 8.2-13.8 10.2-15.5 2 1.7 5.7 5.8 10.2 15.5 6.2 13.4 11.1 32.5 12.7 55l-45.8 0c1.6-22.5 6.5-41.6 12.7-55zm96.7 55L247 176c-1.4-23.1-6-44.2-12.6-61.2 22.7 12.5 39.4 34.8 44.2 61.2zM137 176l-31.6 0c4.8-26.4 21.5-48.7 44.2-61.2-6.7 17-11.2 38-12.6 61.2zm183 16a128 128 0 1 0 -256 0 128 128 0 1 0 256 0z";
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
    exports.faPassport = exports.definition;
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

// src/_entries/faPassport.js
var import_faPassport = __toESM(require_faPassport());
var export_faPassport = import_faPassport.definition;
export {
  export_faPassport as faPassport
};
