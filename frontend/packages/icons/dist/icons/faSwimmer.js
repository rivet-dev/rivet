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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSwimming.js
var require_faPersonSwimming = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSwimming.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-swimming";
    var width = 640;
    var height = 512;
    var aliases = [127946, "swimmer"];
    var unicode = "f5c4";
    var svgPathData = "M552 152a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zM293.4 198.2l-88.6 73.9c1.1 0 2.2-.1 3.3-.1 33.1-.2 66.3 10.2 94.4 31.4 22.1 16.6 29.1 16.6 51.2 0 27.5-20.7 59.9-31.2 92.4-31.4 4.8 0 9.7 .2 14.5 .6-8.3-30-24.3-57.7-46.8-80.2-18.4-18.4-40.6-32.7-65-41.8l-68.6-25.7c-27.4-10.3-58-7.5-83.1 7.6l-53.5 32.1c-15.2 9.1-20.1 28.7-11 43.9s28.7 20.1 43.9 11L230 187.3c8.4-5 18.6-5.9 27.7-2.5l35.7 13.4zm110 181.9c21.3-16.1 49.9-16.1 71.2 0 19 14.4 41.9 28.2 67.2 33.3 26.5 5.4 54.3 .8 80.7-19.1 10.6-8 12.7-23 4.7-33.6s-23-12.7-33.6-4.7c-14.9 11.2-28.6 13.1-42.3 10.3-14.9-3-30.9-11.9-47.8-24.6-38.4-29-90.5-29-129 0-24 18.1-40.7 26.3-54.5 26.3s-30.5-8.2-54.5-26.3c-38.4-29-90.5-29-129 0-21.6 16.3-41.3 25.8-58.9 25.7-9.6-.1-19.9-3-31.2-11.5-10.6-8-25.6-5.9-33.6 4.7S7 386.3 17.6 394.3c19.1 14.4 39.4 21 59.8 21.1 33.9 .2 64.3-17.4 88.1-35.3 21.3-16.1 49.9-16.1 71.2 0 24.2 18.3 52.3 35.9 83.4 35.9s59.1-17.7 83.4-35.9z";
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
    exports.faPersonSwimming = exports.definition;
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

// src/_entries/faSwimmer.js
var import_faPersonSwimming = __toESM(require_faPersonSwimming());
var export_faSwimmer = import_faPersonSwimming.definition;
export {
  export_faSwimmer as faSwimmer
};
