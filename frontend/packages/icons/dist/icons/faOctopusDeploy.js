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

// src/node_modules/@fortawesome/free-brands-svg-icons/faOctopusDeploy.js
var require_faOctopusDeploy = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faOctopusDeploy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "octopus-deploy";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e082";
    var svgPathData = "M455.7 349.2c-45.9-39.1-36.7-77.9-16.1-128.1 35.7-87-23.5-186.9-109.6-212.8-92.9-27.9-195.7 16-230.3 108.8-8.7 23.5-12.5 48.5-11 73.5 1.7 29.5 14.7 53 24.1 80.3 17.2 50.2-28.1 92.7-66.7 117.6-46.8 30.2-36.3 39.9-8.4 41.9 23.4 1.7 44.5-4.5 65.3-15 9.2-4.6 40.7-18.9 45.1-28.6-12.2 26.6-37 72.7-21.5 102.1 19.1 36.2 67.1-31.8 76.7-45.8 8.6-12.6 43-81.3 63.6-46.9 18.9 31.4 8.6 76.4 35.7 104.6 32.9 34.2 51.2-18.3 51.4-44.2 .2-16.4-6.1-95.9 29.9-59.9 21.4 21.4 52.9 71.2 88.6 67 38.7-4.5-22.1-68-28.3-78.7 5.4 4.3 53.7 34.1 53.8 9.5 .1-18.8-30.1-34.7-42.5-45.3z";
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
    exports.faOctopusDeploy = exports.definition;
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

// src/_entries/faOctopusDeploy.js
var import_faOctopusDeploy = __toESM(require_faOctopusDeploy());
var export_faOctopusDeploy = import_faOctopusDeploy.definition;
export {
  export_faOctopusDeploy as faOctopusDeploy
};
