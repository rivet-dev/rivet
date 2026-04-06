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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDryerHeat.js
var require_faDryerHeat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDryerHeat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dryer-heat";
    var width = 448;
    var height = 512;
    var aliases = ["dryer-alt"];
    var unicode = "f862";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 448c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-384c0-35.3-28.7-64-64-64L64 0zM224 160a144 144 0 1 1 0 288 144 144 0 1 1 0-288zM64 88a24 24 0 1 1 48 0A24 24 0 1 1 64 88zM184 64a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM156.7 212.7c-6.2 6.2-6.2 16.4 0 22.6L177.4 256c4.2 4.2 6.6 10 6.6 16s-2.4 11.8-6.6 16l-9.4 9.4c-10.2 10.2-16 24.1-16 38.6s5.8 28.4 16 38.6l20.7 20.7c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6L190.6 352c-4.2-4.2-6.6-10-6.6-16s2.4-11.8 6.6-16l9.4-9.4c10.2-10.2 16-24.1 16-38.6s-5.8-28.4-16-38.6l-20.7-20.7c-6.2-6.2-16.4-6.2-22.6 0zm102.6 0c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L257.4 256c4.2 4.2 6.6 10 6.6 16s-2.4 11.8-6.6 16l-9.4 9.4c-10.2 10.2-16 24.1-16 38.6s5.8 28.4 16 38.6l20.7 20.7c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6L270.6 352c-4.2-4.2-6.6-10-6.6-16s2.4-11.8 6.6-16l9.4-9.4c10.2-10.2 16-24.1 16-38.6s-5.8-28.4-16-38.6l-20.7-20.7z";
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
    exports.faDryerHeat = exports.definition;
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

// src/_entries/faDryerHeat.js
var import_faDryerHeat = __toESM(require_faDryerHeat());
var export_faDryerHeat = import_faDryerHeat.definition;
export {
  export_faDryerHeat as faDryerHeat
};
