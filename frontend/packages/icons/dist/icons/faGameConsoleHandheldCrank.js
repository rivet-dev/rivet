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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGameConsoleHandheldCrank.js
var require_faGameConsoleHandheldCrank = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGameConsoleHandheldCrank.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "game-console-handheld-crank";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5b9";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm96 0c-17.7 0-32 14.3-32 32l0 96c0 17.7 14.3 32 32 32l256 0c17.7 0 32-14.3 32-32l0-96c0-17.7-14.3-32-32-32L96 96zm32 240c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 16-16 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0 0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16 16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-16 0 0-16zm144 24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm56 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM480 168c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 32c0 13.3-10.7 24-24 24l-24 0 0 72c0 13.3-10.7 24-24 24l-24 0 0-48 0-16 0-56 0-32z";
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
    exports.faGameConsoleHandheldCrank = exports.definition;
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

// src/_entries/faGameConsoleHandheldCrank.js
var import_faGameConsoleHandheldCrank = __toESM(require_faGameConsoleHandheldCrank());
var export_faGameConsoleHandheldCrank = import_faGameConsoleHandheldCrank.definition;
export {
  export_faGameConsoleHandheldCrank as faGameConsoleHandheldCrank
};
