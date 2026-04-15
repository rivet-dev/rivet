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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFamilyDress.js
var require_faFamilyDress = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFamilyDress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "family-dress";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e301";
    var svgPathData = "M128 0a64 64 0 1 1 0 128A64 64 0 1 1 128 0zM112.3 160l31.4 0c11.8 0 23 3.2 32.6 8.9c-.2 2.3-.3 4.7-.3 7.1c0 24.3 10.9 46.1 28 60.8c-35.7 18.7-60 56.1-60 99.2l0 16c0 28.4 12.4 54 32 71.6l0 56.4c0 6.1 .8 11.9 2.4 17.5c-8.7 9-20.9 14.5-34.4 14.5l-32 0c-26.5 0-48-21.5-48-48l0-80-21.6 0c-21.3 0-36.6-20.3-30.8-40.8L50.7 206.4c7.8-27.5 33-46.4 61.5-46.4zM368 512c-13.5 0-25.7-5.6-34.4-14.5c1.6-5.5 2.4-11.4 2.4-17.5l0-56.4c19.6-17.6 32-43.1 32-71.6l0-16c0-43.1-24.3-80.5-60-99.2c17.1-14.7 28-36.5 28-60.8c0-2.4-.1-4.8-.3-7.1c9.6-5.7 20.8-8.9 32.6-8.9l31.4 0c28.6 0 53.7 18.9 61.5 46.4l39.1 136.8c5.8 20.4-9.5 40.8-30.8 40.8L448 384l0 80c0 26.5-21.5 48-48 48l-32 0zM384 0a64 64 0 1 1 0 128A64 64 0 1 1 384 0zM256 128a48 48 0 1 1 0 96 48 48 0 1 1 0-96zm0 128c44.2 0 80 35.8 80 80l0 16c0 23.7-12.9 44.4-32 55.4l0 72.6c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-72.6c-19.1-11.1-32-31.7-32-55.4l0-16c0-44.2 35.8-80 80-80z";
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
    exports.faFamilyDress = exports.definition;
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

// src/_entries/faFamilyDress.js
var import_faFamilyDress = __toESM(require_faFamilyDress());
var export_faFamilyDress = import_faFamilyDress.definition;
export {
  export_faFamilyDress as faFamilyDress
};
