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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTableTennisPaddleBall.js
var require_faTableTennisPaddleBall = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTableTennisPaddleBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "table-tennis-paddle-ball";
    var width = 512;
    var height = 512;
    var aliases = [127955, "ping-pong-paddle-ball", "table-tennis"];
    var unicode = "f45d";
    var svgPathData = "M97 127l67.4-67.4c38.2-38.2 90-59.6 144-59.6 112.5 0 203.7 91.2 203.7 203.6 0 46.4-15.8 91.1-44.5 127-23.6-16.8-52.4-26.7-83.5-26.7-31.1 0-59.9 9.9-83.4 26.6L97 127zM240 448c0 9.7 1 19.1 2.8 28.2-19.8-5.2-38-15.5-52.7-30.2-12.2-12.2-31.9-12.2-44.1 0L96.6 495.4c-10.6 10.6-25 16.6-40 16.6-31.2 0-56.6-25.3-56.6-56.6 0-15 6-29.4 16.6-40l49.4-49.4c12.2-12.2 12.2-31.9 0-44.1-21.7-21.7-33.9-51.2-33.9-81.9 0-29.4 11.1-57.6 31.1-79L266.6 364.6C249.9 388.1 240 416.9 240 448zm144-96a96 96 0 1 1 0 192 96 96 0 1 1 0-192z";
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
    exports.faTableTennisPaddleBall = exports.definition;
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

// src/_entries/faTableTennis.js
var import_faTableTennisPaddleBall = __toESM(require_faTableTennisPaddleBall());
var export_faTableTennis = import_faTableTennisPaddleBall.definition;
export {
  export_faTableTennis as faTableTennis
};
