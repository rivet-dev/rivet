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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserNurseHair.js
var require_faUserNurseHair = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserNurseHair.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-nurse-hair";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e45d";
    var svgPathData = "M96 70.2L96 128l.3 0c-.2 2.6-.3 5.3-.3 8l0 40c0 70.7 57.3 128 128 128s128-57.3 128-128l0-40c0-2.7-.1-5.4-.3-8l.3 0 0-57.8c0-13.3-8.3-25.3-20.8-30l-96-36c-7.2-2.7-15.2-2.7-22.5 0l-96 36C104.3 44.9 96 56.8 96 70.2zM304 160l0 16c0 44.2-35.8 80-80 80s-80-35.8-80-80l32 0c29.8 0 55.9-16.3 69.6-40.5C257.3 150.4 275.5 160 296 160l8 0zM141.3 336.2c-7.6-8.1-18.9-11.9-29.4-8.5C47 348.5 0 409.4 0 481.3c0 17 13.8 30.7 30.7 30.7l386.6 0c17 0 30.7-13.8 30.7-30.7c0-71.8-47-132.7-111.9-153.6c-10.5-3.4-21.8 .4-29.4 8.5l-71 75.5c-6.3 6.7-17 6.7-23.3 0l-71-75.5zM208 48c0-4.4 3.6-8 8-8l16 0c4.4 0 8 3.6 8 8l0 16 16 0c4.4 0 8 3.6 8 8l0 16c0 4.4-3.6 8-8 8l-16 0 0 16c0 4.4-3.6 8-8 8l-16 0c-4.4 0-8-3.6-8-8l0-16-16 0c-4.4 0-8-3.6-8-8l0-16c0-4.4 3.6-8 8-8l16 0 0-16z";
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
    exports.faUserNurseHair = exports.definition;
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

// src/_entries/faUserNurseHair.js
var import_faUserNurseHair = __toESM(require_faUserNurseHair());
var export_faUserNurseHair = import_faUserNurseHair.definition;
export {
  export_faUserNurseHair as faUserNurseHair
};
