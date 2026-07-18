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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonRunningFast.js
var require_faPersonRunningFast = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonRunningFast.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-running-fast";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5ff";
    var svgPathData = "M368 0a48 48 0 1 1 0 96 48 48 0 1 1 0-96zM259.2 160c-14.1 0-27.5 5.6-37.5 15.5l-23.1 23.1c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l23.1-23.1C198.4 108.3 228.2 96 259.2 96c11.7 0 23.3 1.7 34.4 5.2l48.7 15c36.4 11.2 65.9 38 80.5 73.1l6.3 15c5 11.9 16.6 19.7 29.5 19.7l21.3 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-21.3 0c-38.8 0-73.7-23.3-88.6-59.1l-5.1-12.3L334 299l56.7 35.4c18.1 11.3 26.6 33.3 20.7 53.9L382.8 488.8c-4.9 17-22.6 26.8-39.6 22s-26.8-22.6-22-39.6l25.4-88.8-86.2-53.9c-25.1-15.7-36.1-46.5-26.8-74.5l31.2-93.7c-1.9-.2-3.7-.3-5.6-.3zM205.5 310.7c7.4 18.1 20.4 33.9 37.9 44.9l9.5 6-11.5 25.9c-7.7 17.3-24.9 28.5-43.9 28.5L128 416c-17.7 0-32-14.3-32-32s14.3-32 32-32l59.2 0 18.3-41.3zM16 95.8l128 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-128 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zm-16 80c0-8.8 7.2-16 16-16l64 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-64 0c-8.8 0-16-7.2-16-16zm16 48l128 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-128 0c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faPersonRunningFast = exports.definition;
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

// src/_entries/faPersonRunningFast.js
var import_faPersonRunningFast = __toESM(require_faPersonRunningFast());
var export_faPersonRunningFast = import_faPersonRunningFast.definition;
export {
  export_faPersonRunningFast as faPersonRunningFast
};
