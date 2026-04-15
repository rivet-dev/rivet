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

// src/node_modules/@fortawesome/free-solid-svg-icons/faShuttleSpace.js
var require_faShuttleSpace = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faShuttleSpace.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shuttle-space";
    var width = 576;
    var height = 512;
    var aliases = ["space-shuttle"];
    var unicode = "f197";
    var svgPathData = "M0 368l0 64c0 26.5 21.5 48 48 48l50 0c40.6 0 80.4-11 115.2-31.9l133.5-80.1-93.3 0-64.9 38.9c-14 8.4-28.9 14.7-44.5 19l0-66.4c9.7-5.6 17.8-13.7 23.4-23.4l198.8 0c67.1 0 127.8-20.6 170.3-70.6 4.6-5.4 4.6-13.3 0-18.8-42.5-50.1-103.2-70.6-170.3-70.6l-198.8 0c-5.6-9.7-13.7-17.8-23.4-23.4l0-66.4c15.6 4.2 30.6 10.6 44.5 19l64.9 38.9 93.3 0-133.5-80.1C178.4 43 138.6 32 98 32L48 32C21.5 32 0 53.5 0 80L0 368zm96 0l0 64-48 0 0-64 48 0zM96 80l0 64-48 0 0-64 48 0zM416 224c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16z";
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
    exports.faShuttleSpace = exports.definition;
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

// src/_entries/faShuttleSpace.js
var import_faShuttleSpace = __toESM(require_faShuttleSpace());
var export_faShuttleSpace = import_faShuttleSpace.definition;
export {
  export_faShuttleSpace as faShuttleSpace
};
